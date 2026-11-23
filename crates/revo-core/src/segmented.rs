//! Parallel HTTP range-download support.
use futures_util::future::try_join_all;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

use crate::{download_to_file, DownloadError, DownloadReport};

#[derive(Debug, Clone)]
pub struct SegmentedDownloadOptions {
    /// Maximum number of simultaneous HTTP range requests.
    pub connections: usize,
    /// Do not split files smaller than this many bytes per segment.
    pub min_segment_size: u64,
    /// Number of retries for transient network and server errors per segment.
    pub retries: usize,
}

impl Default for SegmentedDownloadOptions {
    fn default() -> Self {
        Self {
            connections: 4,
            min_segment_size: 1_048_576,
            retries: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentedDownloadReport {
    pub destination: PathBuf,
    pub bytes_written: u64,
    pub total_bytes: Option<u64>,
    pub segments: usize,
    pub used_fallback: bool,
}

#[derive(Debug, Error)]
pub enum SegmentedDownloadError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("server returned an invalid range response")]
    InvalidRangeResponse,
    #[error("server returned a retryable status: {0}")]
    TransientHttpStatus(reqwest::StatusCode),
    #[error("segment length mismatch: expected {expected} bytes, received {actual}")]
    SegmentLengthMismatch { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteRange {
    start: u64,
    end: u64,
}

impl ByteRange {
    fn len(self) -> u64 {
        self.end - self.start + 1
    }
}

/// Downloads a resource using parallel byte ranges when the server advertises
/// byte-range support and the resource is large enough. Existing partial files,
/// unsupported servers, and small resources use the safe sequential engine.
pub async fn download_segmented_to_file(
    client: &reqwest::Client,
    url: &str,
    destination: impl AsRef<Path>,
    options: &SegmentedDownloadOptions,
) -> Result<SegmentedDownloadReport, SegmentedDownloadError> {
    let destination = destination.as_ref().to_path_buf();
    let part_path = append_suffix(&destination, ".part");
    let has_partial = match fs::metadata(&part_path).await {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    let connections = options.connections.max(1);

    if has_partial || connections == 1 {
        return sequential_fallback(client, url, &destination).await;
    }

    let head = match client.head(url).send().await {
        Ok(response) if response.status().is_success() => response,
        _ => return sequential_fallback(client, url, &destination).await,
    };
    let total = match head
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        Some(length) if length > 0 => length,
        _ => return sequential_fallback(client, url, &destination).await,
    };
    // Pin all parallel requests to the same representation when the server
    // supplies a strong ETag, or fall back to Last-Modified.
    let validator = head
        .headers()
        .get(reqwest::header::ETAG)
        .filter(|value| {
            value
                .to_str()
                .is_ok_and(|tag| !tag.trim_start().starts_with("W/"))
        })
        .cloned()
        .map(|value| (reqwest::header::ETAG, value))
        .or_else(|| {
            head.headers()
                .get(reqwest::header::LAST_MODIFIED)
                .cloned()
                .map(|value| (reqwest::header::LAST_MODIFIED, value))
        });
    let accepts_ranges = head
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("bytes"))
        });
    if !accepts_ranges || total < options.min_segment_size.saturating_mul(2) {
        return sequential_fallback(client, url, &destination).await;
    }

    let ranges = split_ranges(
        total,
        connections.min((total / options.min_segment_size.max(1)).max(1) as usize),
    );
    if ranges.len() < 2 {
        return sequential_fallback(client, url, &destination).await;
    }

    let jobs = ranges.iter().enumerate().map(|(index, range)| {
        fetch_segment(
            client,
            url,
            range_path(&part_path, index),
            *range,
            total,
            validator.clone(),
            options.retries,
        )
    });
    let results = match try_join_all(jobs).await {
        Ok(results) => results,
        Err(error) => {
            let should_fallback = matches!(&error, SegmentedDownloadError::InvalidRangeResponse);
            for index in 0..ranges.len() {
                let _ = fs::remove_file(range_path(&part_path, index)).await;
            }
            if should_fallback {
                return sequential_fallback(client, url, &destination).await;
            }
            return Err(error);
        }
    };

    let mut output = File::create(&part_path).await?;
    let mut bytes_written = 0_u64;
    for (path, expected) in &results {
        let mut input = File::open(path).await?;
        let copied = tokio::io::copy(&mut input, &mut output).await?;
        if copied != *expected {
            return Err(SegmentedDownloadError::SegmentLengthMismatch {
                expected: *expected,
                actual: copied,
            });
        }
        bytes_written += copied;
    }
    output.flush().await?;
    output.sync_all().await?;
    drop(output);
    for (path, _) in &results {
        fs::remove_file(path).await?;
    }
    fs::rename(&part_path, &destination).await?;

    Ok(SegmentedDownloadReport {
        destination,
        bytes_written,
        total_bytes: Some(total),
        segments: ranges.len(),
        used_fallback: false,
    })
}

async fn sequential_fallback(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
) -> Result<SegmentedDownloadReport, SegmentedDownloadError> {
    let report: DownloadReport = download_to_file(client, url, destination).await?;
    Ok(SegmentedDownloadReport {
        destination: report.destination,
        bytes_written: report.bytes_written,
        total_bytes: report.total_bytes,
        segments: 1,
        used_fallback: true,
    })
}

async fn fetch_segment(
    client: &reqwest::Client,
    url: &str,
    path: PathBuf,
    range: ByteRange,
    total: u64,
    validator: Option<(reqwest::header::HeaderName, reqwest::header::HeaderValue)>,
    retries: usize,
) -> Result<(PathBuf, u64), SegmentedDownloadError> {
    let mut attempt = 0;
    loop {
        match fetch_segment_once(client, url, &path, range, total, validator.as_ref()).await {
            Ok(received) => return Ok((path, received)),
            Err(error) => {
                let retryable =
                    matches!(
                        &error,
                        SegmentedDownloadError::Request(request)
                            if request.is_timeout() || request.is_connect()
                    ) || matches!(&error, SegmentedDownloadError::TransientHttpStatus(_));
                let _ = fs::remove_file(&path).await;
                if !retryable || attempt >= retries {
                    return Err(error);
                }
                attempt += 1;
                let delay = std::time::Duration::from_millis(
                    250_u64.saturating_mul(1_u64 << attempt.min(6)),
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

async fn fetch_segment_once(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    range: ByteRange,
    total: u64,
    validator: Option<&(reqwest::header::HeaderName, reqwest::header::HeaderValue)>,
) -> Result<u64, SegmentedDownloadError> {
    let mut request = client
        .get(url)
        .header(
            reqwest::header::RANGE,
            format!("bytes={}-{}", range.start, range.end),
        );
    if let Some((_, value)) = validator {
        request = request.header(reqwest::header::IF_RANGE, value);
    }
    let response = request.send().await?;
    if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        if response.status().is_server_error()
            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return Err(SegmentedDownloadError::TransientHttpStatus(
                response.status(),
            ));
        }
        return Err(SegmentedDownloadError::InvalidRangeResponse);
    }
    let content_range = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .ok_or(SegmentedDownloadError::InvalidRangeResponse)?;
    if parse_content_range(content_range) != Some((range.start, range.end, total)) {
        return Err(SegmentedDownloadError::InvalidRangeResponse);
    }

    let mut file = File::create(path).await?;
    let mut stream = response.bytes_stream();
    let mut received = 0_u64;
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        received += chunk.len() as u64;
        if received > range.len() {
            return Err(SegmentedDownloadError::SegmentLengthMismatch {
                expected: range.len(),
                actual: received,
            });
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    file.sync_all().await?;
    if received != range.len() {
        return Err(SegmentedDownloadError::SegmentLengthMismatch {
            expected: range.len(),
            actual: received,
        });
    }
    Ok(received)
}

fn split_ranges(total: u64, count: usize) -> Vec<ByteRange> {
    if total == 0 || count == 0 {
        return Vec::new();
    }
    let count = (count as u64).min(total) as usize;
    let base = total / count as u64;
    let remainder = total % count as u64;
    let mut start = 0_u64;
    (0..count)
        .map(|index| {
            let length = base + if (index as u64) < remainder { 1 } else { 0 };
            let range = ByteRange {
                start,
                end: start + length - 1,
            };
            start = range.end + 1;
            range
        })
        .collect()
}

fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let value = value.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, total.parse().ok()?))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn range_path(part_path: &Path, index: usize) -> PathBuf {
    append_suffix(part_path, &format!(".segment-{index}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_file_into_contiguous_non_overlapping_ranges() {
        let ranges = split_ranges(10, 3);
        assert_eq!(
            ranges,
            vec![
                ByteRange { start: 0, end: 3 },
                ByteRange { start: 4, end: 6 },
                ByteRange { start: 7, end: 9 },
            ]
        );
        assert_eq!(ranges.iter().map(|range| range.len()).sum::<u64>(), 10);
    }

    #[test]
    fn does_not_create_empty_segments() {
        assert_eq!(split_ranges(2, 4).len(), 2);
        assert!(split_ranges(0, 4).is_empty());
    }

    #[test]
    fn parses_and_validates_content_range_components() {
        assert_eq!(parse_content_range("bytes 10-19/100"), Some((10, 19, 100)));
        assert_eq!(parse_content_range("bytes 10-19/*"), None);
        assert_eq!(parse_content_range("items 10-19/100"), None);
    }
}
