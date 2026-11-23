use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use revo_core::{download_segmented_to_file, SegmentedDownloadOptions};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "revo-d", version, about = "REVO D download manager")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download an HTTP(S) URL to a local file, resuming an existing .part file.
    Download {
        url: String,
        #[arg(short, long)]
        output: PathBuf,
        /// Maximum simultaneous HTTP range requests (set to 1 to disable segmentation).
        #[arg(short = 'c', long, default_value_t = 4)]
        connections: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Command::Download {\n            url,\n            output,\n            connections,\n        } => {
            let client = reqwest::Client::builder()
                .user_agent(concat!("REVO-D/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("failed to initialize HTTP client")?;
            let report = download_segmented_to_file(
                &client,
                &url,
                &output,
                &SegmentedDownloadOptions {\n                    connections,\n                    ..Default::default()\n                },
            )
            .await
            .with_context(|| format!("download failed: {url}"))?;
            println!(
                "Saved {} bytes to {} using {} segment(s){}",
                report.bytes_written,
                report.destination.display(),
                report.segments,
                if report.used_fallback {\n                    " (sequential fallback)"\n                } else {\n                    ""\n                }
            );
        }
    }
    Ok(())
}
