# REVO D architecture
## Initial implementation
- Core: Rust library (revo-core) owns transfer behavior and remains independent of the UI.
- CLI: revo-d is the first executable host and provides a reproducible way to exercise the engine.
- Transport: reqwest with rustls provides HTTP/HTTPS.
- I/O: response bodies are streamed to a sibling .part file; successful transfers are flushed, synced, and renamed into place.
## Build
- Development: cargo build --workspace
- Tests: cargo test --workspace
- Windows x64 release: cargo build --release --workspace
GitHub Actions builds revo-d.exe on Windows and publishes it as a workflow artifact. This is a command-line foundation, not yet the planned Windows 11 graphical application.
## Reliability and safety boundaries
- Existing destination files may be replaced by final rename depending on platform semantics; choose output paths deliberately.
- Failed transfers retain partial data but automatic resume is not implemented yet.
- No DRM, paywall, CAPTCHA, or access-control circumvention is implemented.
