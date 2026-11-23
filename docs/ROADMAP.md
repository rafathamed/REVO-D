# REVO D — Engineering Roadmap

This roadmap separates foundational reliability from advanced protocol and integration work. A feature is not considered complete until it has automated tests, documented limitations, and platform validation where applicable.

## Phase 0 — Repository and engineering foundation

- [x] Initialize repository documentation and ignore rules
- [ ] Choose implementation language, UI framework, and supported Windows baseline
- [ ] Add CI for formatting, linting, unit tests, and dependency auditing
- [ ] Define architecture decision records and contribution workflow

## Phase 1 — Download engine reliability

- [ ] HTTP/HTTPS single-stream downloads
- [ ] Resume using validators (ETag / Last-Modified) and verified range support
- [ ] Atomic writes and crash-safe persisted metadata
- [ ] Retry policy with bounded backoff and cancellation
- [ ] Concurrent segmented downloads with server capability detection
- [ ] Segment scheduling, integrity checks, and safe single-stream fallback
- [ ] Integration tests using controllable local HTTP servers

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
