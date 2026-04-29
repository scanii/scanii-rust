# scanii

Official Rust SDK for the [Scanii](https://www.scanii.com) content security API.

[![crates.io](https://img.shields.io/crates/v/scanii.svg)](https://crates.io/crates/scanii)
[![docs.rs](https://docs.rs/scanii/badge.svg)](https://docs.rs/scanii)

## SDK Principles

1. **Light.** Zero runtime dependencies, stdlib only.
2. **Up to date.** Always current with the latest Scanii API.
3. **Integration-only.** Wraps the REST API — retries, concurrency, and batching are the caller's responsibility.

### About "Light" in Rust

The Rust standard library has no HTTP client and no TLS. True zero-dep is impossible for an HTTPS SDK. This crate uses the smallest viable dependency set:

- [`ureq`](https://crates.io/crates/ureq) — synchronous HTTP client, designed for minimal deps. Pulls `rustls` transitively via the `tls` feature.
- [`serde`](https://crates.io/crates/serde) + [`serde_json`](https://crates.io/crates/serde_json) — the de-facto JSON standard.

That's the entire runtime dependency surface. No `tokio`, no `hyper`, no `reqwest`, no `openssl`. `rustls` is intentionally not a direct dependency — it is pulled by ureq's `tls` feature so the version stays in lockstep with ureq's transitive choice.

## Install

```bash
cargo add scanii
```

Targets current stable Rust.

## Quickstart

```rust,no_run
use scanii::ScaniiClient;

fn main() -> Result<(), scanii::ScaniiError> {
    let client = ScaniiClient::builder()
        .key("your-key")
        .secret("your-secret")
        .build()?;

    // Scan a file from disk:
    let result = client.process_file(std::path::Path::new("./file.pdf"), None, None)?;
    println!("findings: {:?}", result.findings);
    Ok(())
}
```

`findings` is a `Vec<String>`. An empty vector means the content is clean.

A runnable version is at `examples/quickstart.rs` — invoke with:

```bash
SCANII_KEY=key SCANII_SECRET=secret SCANII_ENDPOINT=http://localhost:4000 \
  cargo run --example quickstart -- ./some-file.txt
```

### Scanning from any `Read` source

```rust,no_run
use scanii::ScaniiClient;
use std::io::Cursor;

# fn main() -> Result<(), scanii::ScaniiError> {
let client = ScaniiClient::builder()
    .key("your-key")
    .secret("your-secret")
    .build()?;

// From a file path (most common):
let result = client.process_file(std::path::Path::new("./file.pdf"), None, None)?;

// From any std::io::Read source — in-memory buffers, network streams, etc.:
let bytes: &[u8] = b"hello world";
let result = client.process(
    Cursor::new(bytes),
    "hello.txt",
    Some("text/plain"),
    None,
    None,
)?;
# Ok(()) }
```

`process` and `process_async` accept any `impl Read` and stream the body without buffering, so memory use is independent of content length.

## API

| Method | REST | Returns |
|---|---|---|
| `process(reader, filename, content_type, metadata, callback)` | `POST /files` | `Result<ScaniiProcessingResult>` |
| `process_file(path, metadata, callback)` | `POST /files` | `Result<ScaniiProcessingResult>` |
| `process_async(reader, filename, content_type, metadata, callback)` | `POST /files/async` | `Result<ScaniiPendingResult>` |
| `process_async_file(path, metadata, callback)` | `POST /files/async` | `Result<ScaniiPendingResult>` |
| `fetch(url, metadata, callback)` | `POST /files/fetch` | `Result<ScaniiPendingResult>` |
| `retrieve(id)` | `GET /files/{id}` | `Result<ScaniiProcessingResult>` |
| `ping()` | `GET /ping` | `Result<()>` |
| `create_auth_token(timeout_seconds)` | `POST /auth/tokens` | `Result<ScaniiAuthToken>` |
| `retrieve_auth_token(id)` | `GET /auth/tokens/{id}` | `Result<ScaniiAuthToken>` |
| `delete_auth_token(id)` | `DELETE /auth/tokens/{id}` | `Result<()>` |

Full API reference: <https://scanii.github.io/openapi/v22/>.

## Regional endpoints

```rust
use scanii::ScaniiClient;
let client = ScaniiClient::builder()
    .key("k")
    .secret("s")
    .endpoint("https://api-eu1.scanii.com")
    .build()
    .unwrap();
```

| Region | Endpoint |
|---|---|
| Auto (default) | `https://api.scanii.com` |
| US 1 | `https://api-us1.scanii.com` |
| EU 1 | `https://api-eu1.scanii.com` |
| EU 2 | `https://api-eu2.scanii.com` |
| AP 1 | `https://api-ap1.scanii.com` |
| AP 2 | `https://api-ap2.scanii.com` |
| CA 1 | `https://api-ca1.scanii.com` |

## Errors

```rust,no_run
use scanii::{ScaniiClient, ScaniiError};

# fn run() -> Result<(), ScaniiError> {
# let client = ScaniiClient::builder().key("k").secret("s").build()?;
match client.ping() {
    Ok(()) => println!("ok"),
    Err(ScaniiError::Auth { message, .. }) => eprintln!("bad creds: {message}"),
    Err(ScaniiError::RateLimit { retry_after, .. }) => {
        eprintln!("rate-limited; retry after {retry_after:?}s");
    }
    Err(e) => eprintln!("other error: {e}"),
}
# Ok(()) }
```

Per SDK Principle 3, the SDK does not retry on the caller's behalf — backoff and retry policy belong to your application.

## Local testing with scanii-cli

The SDK ships integration tests against [scanii-cli](https://github.com/scanii/scanii-cli), a local mock server. No real Scanii credentials are needed.

```bash
docker run -d --name scanii-cli -p 4000:4000 ghcr.io/scanii/scanii-cli:latest server

cargo test
```

Override the endpoint by exporting `SCANII_TEST_ENDPOINT=...` before `cargo test`. Tests self-skip when scanii-cli is not reachable, so `cargo test` is safe to run in any environment.

## Auth tokens

Mint a short-lived token server-side and authenticate with it from a less-trusted client:

```rust,no_run
# use scanii::ScaniiClient;
# fn run() -> Result<(), scanii::ScaniiError> {
let server_client = ScaniiClient::builder().key("k").secret("s").build()?;
let token = server_client.create_auth_token(300)?;

let token_client = ScaniiClient::builder().token(&token.id).build()?;
token_client.ping()?;
# Ok(()) }
```

## Contributing

Bug reports and PRs welcome at <https://github.com/scanii/scanii-rust/issues>.

## License

[Apache-2.0](LICENSE).
