# REVO D architecture

## Current implementation

- **Core:** Rust library (`revo-core`) owns transfer behavior and remains independent of the UI.
- **CLI:** `revo-d` is the first executable host and exposes the transfer engine.
- **Transport:** `reqwest` with rustls provides HTTP/HTTPS.
- **Sequential I/O:** response bodies stream to a sibling `.part` file. Successful transfers are flushed, synced, and renamed into place.
- **Resume:** an existing `.part` file triggers an HTTP Range request. A 206 response is checked against the requested starting offset; if the server ignores Range and returns 200, the partial file is safely restarted.
- **Segmentation:** for a new, sufficiently large download, the engine probes with HEAD and checks Content-Length and Accept-Ranges. It splits the resource into bounded, non-overlapping byte ranges and downloads them concurrently to separate temporary files.
- **Segment validation:** every segment must return 206 with an exact Content-Range matching its requested start, end, and total size. The received byte count must also match the range length. Segments are concatenated in range order, synced, and renamed into the destination.
- **Fallback:** unsupported or ignored range requests fall back to the sequential downloader. Existing partial downloads currently use the sequential resume path rather than mixing old partial data with new parallel segments.

## Build

- Development: `cargo build --workspace`
- Formatting: `cargo fmt --all -- --check`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Tests: `cargo test --workspace`
- Windows x64 release: `cargo build --release --workspace`

GitHub Actions validates formatting, linting, unit/integration tests, and builds `revo-d.exe` on Windows. The executable is published as a workflow artifact. This is a command-line foundation, not yet the planned Windows 11 graphical application.

## Current limitations and safety boundaries

- Resume does not yet pin a representation with ETag or Last-Modified validators; a resource that changes between attempts needs stronger validation.
- Segmented downloads validate ranges and lengths, but do not yet provide end-to-end cryptographic integrity verification.
- Retry policy, pause/cancel controls, persisted queue metadata, and GUI remain future work.
- Destination replacement behavior follows platform rename semantics; choose output paths deliberately.
- No DRM, paywall, CAPTCHA, or access-control circumvention is implemented.
