# REVO D architecture

## Initial implementation
- **Core:** Rust library (`revo-core`) owns transfer behavior and remains independent of the UI.
- **CLI:** `revo-d` is the first executable host and provides a reproducible way to exercise the engine.
- **Transport:** `reqwest` with rustls provides HTTP/HTTPS.
- **I/O:** response bodies are streamed to a sibling `.part` file. Successful transfers are flushed, synced, and renamed into place.
- **Resume:** an existing `.part` file triggers an HTTP Range request. A valid 206 response is appended after checking Content-Range; a server that ignores Range and returns 200 causes a safe restart.

## Build
- Development: `cargo build --workspace`
- Tests: `cargo test --workspace`
- Windows x64 release: `cargo build --release --workspace`

GitHub Actions builds `revo-d.exe` on Windows and publishes it as a workflow artifact. This is a command-line foundation, not yet the planned Windows 11 graphical application.

## Reliability and safety boundaries
- Existing destination files may be replaced by the final rename depending on platform semantics; choose output paths deliberately.
- Resume currently depends on the origin server honoring byte ranges and returning a valid Content-Range header.
- Multi-connection segmentation, persisted download metadata, retries, and pause/cancel controls remain planned work.
- No DRM, paywall, CAPTCHA, or access-control circumvention is implemented.
