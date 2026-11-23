use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use revo_core::download_to_file;
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
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Command::Download { url, output } => {
            let client = reqwest::Client::builder()
                .user_agent(concat!("REVO-D/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("failed to initialize HTTP client")?;
            let report = download_to_file(&client, &url, &output)
                .await
                .with_context(|| format!("download failed: {url}"))?;
            println!(
                "Saved {} bytes to {} (resumed from byte {})",
                report.bytes_written,
                report.destination.display(),
                report.resumed_from
            );
        }
    }
    Ok(())
}
