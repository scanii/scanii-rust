# Changelog

All notable changes to the `scanii` crate are documented here. Versions follow [SemVer](https://semver.org).

## 1.0.0 — Initial release

First public release of the Scanii Rust SDK on crates.io.

### API surface

- `ScaniiClient::process(path, metadata, callback)` → `ScaniiProcessingResult`
- `ScaniiClient::process_reader(reader, filename, content_type, metadata, callback)` → `ScaniiProcessingResult`
- `ScaniiClient::process_async(path, metadata, callback)` → `ScaniiPendingResult`
- `ScaniiClient::process_async_reader(reader, filename, content_type, metadata, callback)` → `ScaniiPendingResult`
- `ScaniiClient::fetch(location, metadata, callback)` → `ScaniiPendingResult`
- `ScaniiClient::retrieve(id)` → `ScaniiProcessingResult`
- `ScaniiClient::ping()` → `()`
- `ScaniiClient::create_auth_token(timeout_seconds)` → `ScaniiAuthToken`
- `ScaniiClient::retrieve_auth_token(id)` → `ScaniiAuthToken`
- `ScaniiClient::delete_auth_token(id)` → `()`

Errors: `ScaniiError::Auth` (401/403), `ScaniiError::RateLimit` (429, with `retry_after`), `ScaniiError::Http`, `ScaniiError::Transport`, `ScaniiError::Serde`, `ScaniiError::Io`, `ScaniiError::Config`.

### Highlights

- **Minimal dependencies** — `ureq`, `serde`, `serde_json`. `rustls` pulled transitively via ureq's `tls` feature.
- **Streaming uploads** — `process_reader` and `process_async_reader` accept any `impl Read` source. Memory use is independent of content length.
- **Synchronous** — single-threaded by default; clients are `Send + Sync` and can be shared across threads.
- **Builder-pattern construction** — `ScaniiClient::builder().key(...).secret(...).build()`.
- **API v2.2.**
- **Targets current stable Rust; no MSRV pinned.**
- **scanii-cli** integration tests cover the cross-OS matrix (Linux / macOS / Windows on stable) without burning real Scanii credits.
