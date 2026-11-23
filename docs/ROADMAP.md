# REVO D — Engineering Roadmap

A feature is considered complete only after automated tests, documented limitations, and platform validation where applicable.

## Phase 0 — Repository and engineering foundation

- [x] Initialize repository documentation and ignore rules
- [x] Select Rust for the core and CLI; establish a Windows x64 release target
- [x] Add CI for formatting, linting, tests, and Windows release builds
- [x] Document the current architecture and development workflow
- [ ] Select the Windows GUI framework and supported Windows baseline

## Phase 1 — Download engine reliability

- [x] HTTP/HTTPS single-stream downloads
- [x] Resume from an existing partial file using HTTP Range and validate the returned starting offset
- [x] Stream to a temporary file, flush/sync, and rename on completion
- [x] Concurrent segmented downloads with server capability detection
- [x] Validate each segment's Content-Range and exact received byte count
- [x] Assemble segments in order and fall back to sequential transfer when ranges are unsupported or ignored
- [x] Integration tests using controllable local HTTP servers
- [ ] Pin resumed representations with ETag / Last-Modified validators
- [ ] Add bounded retries, backoff, and cancellation
- [ ] Add crash-safe persisted transfer metadata and stronger end-to-end integrity checks

## Phase 2 — Product foundation

- [ ] Persistent queue, categories, and download history
- [ ] Pause, resume, cancel, retry, and queue priorities
- [ ] Speed limits, per-download limits, and scheduling
- [ ] Proxy configuration and credential-safe secret storage
- [ ] Accessible desktop UI and localization

## Phase 3 — Browser integration

- [ ] Native messaging host with strict origin validation
- [ ] Browser extension capture and duplicate detection
- [ ] Explicit user controls for capture rules and private browsing
- [ ] Installer registration and clean uninstall

## Phase 4 — Advanced capabilities

- [ ] FTP/FTPS and SFTP after protocol-specific security review
- [ ] HLS/DASH support for permitted, non-DRM streams
- [ ] Website mirroring with scope limits and robots/politeness controls
- [ ] Archive inspection and extraction workflow
- [ ] Optional BitTorrent support with configurable privacy controls

## Quality gates

- No claim of IDM parity without reproducible feature-by-feature tests
- No bypass of DRM, CAPTCHA, paywalls, or access controls
- Never log passwords, cookies, authorization headers, or signed URLs
- Test interruption, process crashes, disk-full conditions, and malformed server responses
- Benchmark throughput and resource use against documented test fixtures
