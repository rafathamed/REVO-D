//! REVO D download engine foundation.
//! Downloads are streamed to a sibling .part file and promoted only after success.
use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("server returned unsuccessful status: {0}")]
    HttpStatus(reqwest::StatusCode),
}
#[derive(Debug, Clone)]
pub struct DownloadReport {
    pub destination: PathBuf,
    pub bytes_written: u64,
    pub total_bytes: Option<u64>,
}

/// Stream an HTTP(S) resource to disk without buffering it all in memory.
/// Failed transfers leave a .part file for future resume support.
pub async fn download_to_file(
    client: &reqwest::Client, url: &str, destination: impl AsRef<Path>,
) -> Result<DownloadReport, DownloadError> {
    let destination = destination.as_ref().to_path_buf();
    let part_path = append_suffix(&destination, ".part");
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus(response.status()));
    }
    let total_bytes = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = File::create(&part_path).await?;
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
    Ok(DownloadReport { destination, bytes_written, total_bytes })
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
        assert_eq!(append_suffix(Path::new("archive.zip"), ".part"), PathBuf::from("archive.zip.part"));
    }
}
