//! REVO D download engine foundation.
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("server returned unsuccessful status: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("server returned an invalid Content-Range header")]
    InvalidContentRange,
}

#[derive(Debug, Clone)]
pub struct DownloadReport {
    pub destination: PathBuf,
    pub bytes_written: u64,
    pub total_bytes: Option<u64>,
    pub resumed_from: u64,
}

/// Download an HTTP(S) resource, resuming from an existing .part file when
/// the server supports byte ranges. If the server ignores Range (HTTP 200),
/// the partial file is safely restarted rather than appended incorrectly.
pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    destination: impl AsRef<Path>,
) -> Result<DownloadReport, DownloadError> {
    let destination = destination.as_ref().to_path_buf();
    let part_path = append_suffix(&destination, ".part");
    let offset = match tokio::fs::metadata(&part_path).await {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error.into()),
    };

    let mut request = client.get(url);
    if offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::HttpStatus(status));
    }

    let (resume_offset, total_bytes, append) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let value = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or(DownloadError::InvalidContentRange)?;
        let (start, total) =
            parse_content_range(value).ok_or(DownloadError::InvalidContentRange)?;
        if start != offset {
            return Err(DownloadError::InvalidContentRange);
        }
        (offset, total, offset > 0)
    } else {
        (0, response.content_length(), false)
    };

    let mut stream = response.bytes_stream();
    let mut file = if append {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part_path)
            .await?
    } else {
        File::create(&part_path).await?
    };
    let mut bytes_written = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        bytes_written += chunk.len() as u64;
    }
    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    tokio::fs::rename(&part_path, &destination).await?;
    Ok(DownloadReport {
        destination,
        bytes_written,
        total_bytes,
        resumed_from: resume_offset,
    })
}

fn parse_content_range(value: &str) -> Option<(u64, Option<u64>)> {
    let value = value.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let (start, _end) = range.split_once('-')?;
    let start = start.parse().ok()?;
    let total = if total == "*" {
        None
    } else {
        Some(total.parse().ok()?)
    };
    Some((start, total))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_preserves_original_extension() {
        assert_eq!(
            append_suffix(Path::new("archive.zip"), ".part"),
            PathBuf::from("archive.zip.part")
        );
    }

    #[test]
    fn parses_content_range_with_known_total() {
        assert_eq!(
            parse_content_range("bytes 10-19/100"),
            Some((10, Some(100)))
        );
    }

    #[test]
    fn parses_content_range_with_unknown_total() {
        assert_eq!(parse_content_range("bytes 10-19/*"), Some((10, None)));
    }

    #[test]
    fn rejects_invalid_content_range() {
        assert_eq!(parse_content_range("items 10-19/100"), None);
        assert_eq!(parse_content_range("bytes nope"), None);
    }
}

mod segmented;
pub use segmented::{
    download_segmented_to_file, SegmentedDownloadError, SegmentedDownloadOptions,
    SegmentedDownloadReport,
};
