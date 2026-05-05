# Changelog

All notable changes to the `scanii` crate are documented here. Versions follow [SemVer](https://semver.org).

## 1.3.0 — deprecate AUTO endpoint

Additive minor release. Backward-compatible.

### New types

- `ScaniiTarget` — typed regional API endpoint. Regional constructors: `ScaniiTarget::us1()`,
  `eu1()`, `eu2()`, `ap1()`, `ap2()`, `ca1()`. Custom URL constructor:
  `ScaniiTarget::from_url(url)`.

### New builder method

- `ScaniiClientBuilder::target(ScaniiTarget)` — preferred way to set the API endpoint.

### Deprecated

- `ScaniiClientBuilder::endpoint(String)` — use `.target(ScaniiTarget::us1())` or
  `.target(ScaniiTarget::from_url(...))` instead. Will be removed in a future major version.
- Constructing a client without calling `.target(...)` or `.endpoint(...)` now emits a
  deprecation warning to stderr at runtime (fallback to `https://api.scanii.com` / AUTO routing).

---

## 1.2.0 — v2.2 API surface

Additive minor release tracking the Scanii v2.2 API.

### New methods

- `ScaniiClient::retrieve_trace(id)` — retrieve ordered processing events for a scan id (`GET /files/{id}/trace`). Returns `Ok(None)` on 404. Preview surface per the v2.2 spec.
- `ScaniiClient::process_from_url(location, metadata)` — synchronous URL submission (`POST /files` with `location` form field). Server fetches and scans the URL, returning a `ScaniiProcessingResult`. Preview surface per the v2.2 spec.

### New types

- `ScaniiTraceResult` — holds `resource_id`, `events: Vec<ScaniiTraceEvent>`, and header-derived fields.
- `ScaniiTraceEvent` — holds `timestamp` (ISO 8601 string) and `message`.

### Deprecated

- `ScaniiProcessingResult::error` — the server never populates this field on a successful response; server-side errors arrive as non-2xx responses and are surfaced via `ScaniiError`. Will be removed in a future major version.

---

## 1.1.0 — Streaming standardization

Aligns method names with the cross-SDK streaming standard.

### New methods

- `ScaniiClient::process<R: Read>(reader, filename, content_type, metadata, callback)` — canonical stream-based method (was `process_reader`)
- `ScaniiClient::process_file(path, metadata, callback)` — path convenience (was `process`)
- `ScaniiClient::process_async<R: Read>(reader, filename, content_type, metadata, callback)` — canonical stream-based (was `process_async_reader`)
- `ScaniiClient::process_async_file(path, metadata, callback)` — path convenience (was `process_async`)

### Deprecated (still functional)

- `process_reader` — use `process` instead; will be removed in a future major version
- `process_path` — use `process_file` instead; will be removed in a future major version
- `process_async_reader` — use `process_async` instead; will be removed in a future major version
- `process_async_path` — use `process_async_file` instead; will be removed in a future major version

---

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
