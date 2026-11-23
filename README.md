# REVO D

A modern, extensible download manager built around a Rust transfer engine.

> Project status: working command-line foundation. The Windows GUI, persistent queue, browser integration, and advanced controls are still under development.

## Current capabilities

- HTTP/HTTPS streaming downloads
- Resume from an existing `.part` file when the server honors byte ranges
- Parallel HTTP range downloads for sufficiently large files
- Per-segment Content-Range and byte-count validation before ordered assembly
- Sequential fallback when range support is unavailable or ignored
- Windows x64 release builds published through GitHub Actions

## Build

Requirements: stable Rust toolchain (Rust 1.78 or newer).

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

## CLI usage

```sh
revo-d download https://example.com/file.zip --output file.zip
```

Set the maximum number of simultaneous range requests with `--connections` (default: 4). Use `--connections 1` to force the sequential engine. Files smaller than the configured per-segment threshold, servers without advertised byte-range support, and existing partial downloads use the sequential path.

Windows x64 builds are available as the `revo-d-windows-x64` artifact in successful GitHub Actions runs.

## Security

Only download content you are authorized to access. REVO D does not attempt to bypass DRM, authentication controls, CAPTCHA, paywalls, or other access restrictions.
