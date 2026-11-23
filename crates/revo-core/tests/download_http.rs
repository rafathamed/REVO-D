use revo_core::{download_to_file, DownloadError};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn destination(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "revo-d-{label}-{}-{}.bin",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn serve_once(body: &'static [u8], honor_range: bool) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("server address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).expect("read request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request_text = String::from_utf8_lossy(&request).to_string();
        let range = request_text.lines().find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("range: bytes=")
                .and_then(|value| value.trim().strip_suffix("-")?.parse::<usize>().ok())
        });

        if honor_range {
            if let Some(start) = range {
                let start = start.min(body.len());
                let payload = &body[start..];
                write!(
                    stream,
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                    payload.len(),
                    start,
                    body.len().saturating_sub(1),
                    body.len()
                )
                .expect("write partial headers");
                stream.write_all(payload).expect("write partial body");
            } else {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write full headers");
                stream.write_all(body).expect("write full body");
            }
        } else {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write full headers");
            stream.write_all(body).expect("write full body");
        }
        request_text
    });
    (format!("http://{address}/file"), handle)
}


fn serve_short_range(
    body: &'static [u8],
    start: usize,
    count: usize,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("server address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).expect("read request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\\r\\n\\r\\n") {
                break;
            }
        }
        let request_text = String::from_utf8_lossy(&request).to_string();
        let end = start + count - 1;
        let payload = &body[start..=end];
        write!(
            stream,
            "HTTP/1.1 206 Partial Content\\r\\nContent-Length: {}\\r\\nContent-Range: bytes {}-{}/{}\\r\\nConnection: close\\r\\n\\r\\n",
            payload.len(),
            start,
            end,
            body.len()
        )
        .expect("write partial headers");
        stream.write_all(payload).expect("write partial body");
        request_text
    });
    (format!("http://{address}/file"), handle)
}

#[tokio::test]
async fn downloads_complete_http_response() {
    let body = b"complete download payload";
    let (url, server) = serve_once(body, false);
    let path = destination("complete");
    let client = reqwest::Client::new();

    let report = download_to_file(&client, &url, &path)
        .await
        .expect("download succeeds");

    assert_eq!(fs::read(&path).expect("read downloaded file"), body);
    assert_eq!(report.bytes_written, body.len() as u64);
    assert_eq!(report.resumed_from, 0);
    server.join().expect("server thread");
    fs::remove_file(path).expect("cleanup destination");
}

#[tokio::test]
async fn resumes_existing_partial_file_when_server_returns_206() {
    let body = b"resumable download payload";
    let split = 10;
    let (url, server) = serve_once(body, true);
    let path = destination("resume");
    fs::write(format!("{}.part", path.display()), &body[..split]).expect("write partial file");
    let client = reqwest::Client::new();

    let report = download_to_file(&client, &url, &path)
        .await
        .expect("resumed download succeeds");

    assert_eq!(fs::read(&path).expect("read downloaded file"), body);
    assert_eq!(report.resumed_from, split as u64);
    assert_eq!(report.bytes_written, (body.len() - split) as u64);
    let request = server.join().expect("server thread");
    assert!(request.to_ascii_lowercase().contains("range: bytes=10-"));
    fs::remove_file(path).expect("cleanup destination");
}

#[tokio::test]
async fn restarts_partial_file_when_server_ignores_range() {
    let body = b"server ignored the range header";
    let (url, server) = serve_once(body, false);
    let path = destination("fallback");
    fs::write(format!("{}.part", path.display()), b"stale partial bytes")
        .expect("write partial file");
    let client = reqwest::Client::new();

    let report = download_to_file(&client, &url, &path)
        .await
        .expect("fallback download succeeds");

    assert_eq!(fs::read(&path).expect("read downloaded file"), body);
    assert_eq!(report.resumed_from, 0);
    assert_eq!(report.bytes_written, body.len() as u64);
    server.join().expect("server thread");
    fs::remove_file(path).expect("cleanup destination");
}


#[tokio::test]
async fn rejects_partial_range_that_does_not_reach_end_of_file() {
    let body = b"resumable download payload";
    let split = 10;
    let (url, server) = serve_short_range(body, split, 5);
    let path = destination("short-range");
    let part_path = PathBuf::from(format!("{}.part", path.display()));
    fs::write(&part_path, &body[..split]).expect("write partial file");

    let result = download_to_file(&reqwest::Client::new(), &url, &path).await;

    assert!(matches!(result, Err(DownloadError::InvalidContentRange)));
    let request = server.join().expect("server thread");
    assert!(request.to_ascii_lowercase().contains("range: bytes=10-"));
    assert!(!path.exists(), "incomplete response must not become final file");
    assert_eq!(fs::read(&part_path).expect("read retained partial"), &body[..split]);
    fs::remove_file(part_path).expect("cleanup partial file");
}
