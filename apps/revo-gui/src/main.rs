#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use revo_core::{download_segmented_to_file, SegmentedDownloadOptions};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct DownloadState {
    running: bool,
    message: String,
    abort: Option<tokio::task::AbortHandle>,
}

struct RevoApp {
    url: String,
    destination: String,
    connections: usize,
    state: Arc<Mutex<DownloadState>>,
}

impl Default for RevoApp {
    fn default() -> Self {
        Self {
            url: String::new(),
            destination: "download.bin".to_owned(),
            connections: 4,
            state: Arc::new(Mutex::new(DownloadState {
                running: false,
                message: "Ready — enter a download URL to begin.".to_owned(),
                abort: None,
            })),
        }
    }
}

impl RevoApp {
    fn start_download(&mut self) {
        let url = self.url.trim().to_owned();
        let destination = self.destination.trim().to_owned();
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            if let Ok(mut state) = self.state.lock() {
                state.message = "Enter a valid HTTP or HTTPS URL.".to_owned();
            }
            return;
        }
        if destination.is_empty() {
            if let Ok(mut state) = self.state.lock() {
                state.message = "Choose a destination file name.".to_owned();
            }
            return;
        }

        let connections = self.connections;
        let shared = Arc::clone(&self.state);
        if let Ok(mut state) = shared.lock() {
            state.running = true;
            state.abort = None;
            state.message = format!("Connecting to {url}");
        }

        std::thread::spawn(move || {
            let result = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => {
                    let task_state = Arc::clone(&shared);
                    runtime.block_on(async move {
                    let task = tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent(concat!("REVO-D/", env!("CARGO_PKG_VERSION")))
                            .build()
                            .map_err(|error| error.to_string())?;
                        download_segmented_to_file(
                            &client,
                            &url,
                            &destination,
                            &SegmentedDownloadOptions {
                                connections,
                                ..Default::default()
                            },
                        )
                        .await
                        .map_err(|error| error.to_string())
                    });

                    if let Ok(mut state) = task_state.lock() {
                        state.abort = Some(task.abort_handle());
                    }

                    match task.await {
                        Ok(result) => result,
                        Err(error) if error.is_cancelled() => {
                            Err("Download cancelled. Any resumable partial data was retained.".to_owned())
                        }
                        Err(error) => Err(format!("Download task failed: {error}")),
                    }
                })
                },
                Err(error) => Err(format!("Could not start the async runtime: {error}")),
            };

            if let Ok(mut state) = shared.lock() {
                state.running = false;
                state.abort = None;
                state.message = match result {
                    Ok(report) => format!(
                        "Completed: {} bytes saved to {} ({} segment(s)).",
                        report.bytes_written,
                        report.destination.display(),
                        report.segments
                    ),
                    Err(error) => format!(" {error}").trim().to_owned(),
                };
            }
        });
    }

    fn cancel_download(&self) {
        let abort = self
            .state
            .lock()
            .ok()
            .and_then(|state| state.abort.clone());
        if let Some(abort) = abort {
            abort.abort();
            if let Ok(mut state) = self.state.lock() {
                state.message = "Cancelling download…".to_owned();
            }
        }
    }
}

impl eframe::App for RevoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (running, message) = self
            .state
            .lock()
            .map(|state| (state.running, state.message.clone()))
            .unwrap_or((false, "Download state unavailable.".to_owned()));

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("REVO D");
                ui.label("Download manager");
            });
            ui.label("Rust-powered HTTP/HTTPS transfers");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.heading("New download");
            ui.add_space(8.0);

            ui.label("Download URL");
            ui.add(
                egui::TextEdit::singleline(&mut self.url)
                    .hint_text("https://example.com/file.zip")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            ui.label("Save as");
            ui.add(
                egui::TextEdit::singleline(&mut self.destination)
                    .hint_text("C:\\Downloads\\file.zip")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Connections");
                ui.add(egui::Slider::new(&mut self.connections, 1..=16));
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!running, egui::Button::new("Start download"))
                    .clicked()
                {
                    self.start_download();
                }
                if ui
                    .add_enabled(running, egui::Button::new("Cancel download"))
                    .clicked()
                {
                    self.cancel_download();
                }
            });

            ui.separator();
            ui.heading("Status");
            ui.label(message);
            if running {
                ui.spinner();
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([560.0, 360.0])
            .with_title("REVO D"),
        ..Default::default()
    };
    eframe::run_native(
        "REVO D",
        options,
        Box::new(|_creation_context| Ok(Box::new(RevoApp::default()))),
    )
}
