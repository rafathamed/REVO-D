use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
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
    // A console executable opened by double-click has no command-line arguments.
    // Keep the window visible and explain the supported usage instead of flashing closed.
    if std::env::args_os().len() == 1 {
        let mut command = Args::command();
        command.print_help()?;
        println!("\n\nREVO D is currently a command-line download engine, not a graphical installer.");
        println!("Example: revo-d download https://example.com/file.zip --output file.zip");
        println!("\nPress Enter to close this window...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        return Ok(());
    }

    match Args::parse().command {
        Command::Download {
            url,
            output,
            connections,
        } => {
            let client = reqwest::Client::builder()
                .user_agent(concat!("REVO-D/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("failed to initialize HTTP client")?;
            let report = download_segmented_to_file(
                &client,
                &url,
                &output,
                &SegmentedDownloadOptions {
                    connections,
                    ..Default::default()
                },
            )
            .await
            .with_context(|| format!("download failed: {url}"))?;
            println!(
                "Saved {} bytes to {} using {} segment(s){}",
                report.bytes_written,
                report.destination.display(),
                report.segments,
                if report.used_fallback {
                    " (sequential fallback)"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}
