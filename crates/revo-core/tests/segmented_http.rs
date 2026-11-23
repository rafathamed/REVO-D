use revo_core::{download_segmented_to_file, SegmentedDownloadOptions};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn destination() -> PathBuf {
    std::env::temp_dir().join(format!(
        "revo-d-segmented-{}-{}.bin",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn serve_ranges(body: Vec<u8>, get_requests: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("server address");
    let body = Arc::new(body);
    let handle = thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..=get_requests {
            let (mut stream, _) = listener.accept().expect("accept request");
            let body = Arc::clone(&body);
            workers.push(thread::spawn(move || {
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let count = stream.read(&mut buffer).expect("read request");
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r
\r
") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request).to_string();
                if request.starts_with("HEAD ") {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r
Content-Length: {}\r
Accept-Ranges: bytes\r
Connection: close\r
\r
",
                        body.len()
                    )
                    .expect("write HEAD response");
                    return;
                }

                let range = request
                    .lines()
                    .find_map(|line| {
                        let value = line.to_ascii_lowercase();
                        let value = value.strip_prefix("range: bytes=")?;
                        let (start, end) = value.trim().split_once('-')?;
                        Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
                    })
                    .expect("range header");
                let (start, end) = range;
                let payload = &body[start..=end];
                write!(
                    stream,
                    "HTTP/1.1 206 Partial Content\r
Content-Length: {}\r
Content-Range: bytes {}-{}/{}\r
Connection: close\r
\r
",
                    payload.len(),
                    start,
                    end,
                    body.len()
                )
                .expect("write range headers");
                stream.write_all(payload).expect("write range body");
            }));
        }
        for worker in workers {
            worker.join().expect("request worker");
        }
    });
    (format!("http://{address}/large-file"), handle)
}

#[tokio::test]
async fn downloads_and_assembles_parallel_ranges_without_corruption() {
    let body: Vec<u8> = (0..(3 * 1024 * 1024 + 17))
        .map(|index| (index % 251) as u8)
        .collect();
    let (url, server) = serve_ranges(body.clone(), 3);
    let path = destination();
    let client = reqwest::Client::new();

    let report = download_segmented_to_file(
        &client,
        &url,
        &path,
        &SegmentedDownloadOptions {
            connections: 3,
            min_segment_size: 1024 * 1024,
        },
    )
    .await
    .expect("segmented download succeeds");

    assert_eq!(fs::read(&path).expect("read final file"), body);
    assert_eq!(report.bytes_written, body.len() as u64);
    assert_eq!(report.total_bytes, Some(body.len() as u64));
    assert_eq!(report.segments, 3);
    assert!(!report.used_fallback);
    server.join().expect("server thread");
    fs::remove_file(path).expect("cleanup destination");
}
