//! Minimal-dependency Rust SDK for the [Scanii](https://www.scanii.com)
//! content security API.
//!
//! See the API reference at <https://scanii.github.io/openapi/v22/>.
//!
//! # Principles
//!
//! 1. **Light.** The smallest viable dependency set for HTTPS in Rust:
//!    `ureq`, `serde`, `serde_json`. `rustls` is pulled transitively by
//!    ureq's `tls` feature — never depended on directly.
//! 2. **Up to date.** Tracks the latest Scanii API.
//! 3. **Integration-only.** Wraps the REST API. Retries, concurrency, and
//!    batching are the caller's responsibility.
//!
//! # Quickstart
//!
//! ```no_run
//! use scanii::ScaniiClient;
//! # fn main() -> Result<(), scanii::ScaniiError> {
//! let client = ScaniiClient::builder()
//!     .key("your-key")
//!     .secret("your-secret")
//!     .build()?;
//!
//! let result = client.process(std::path::Path::new("./file.pdf"), None, None)?;
//! println!("findings: {:?}", result.findings);
//! # Ok(())
//! # }
//! ```
//!
//! # Errors
//!
//! All public methods return [`Result<T, ScaniiError>`]. The error type is
//! an enum: [`ScaniiError::Auth`] for `401/403`, [`ScaniiError::RateLimit`]
//! for `429` (with optional `retry_after`), and [`ScaniiError::Http`] for
//! other non-success statuses.
//!
//! # Local testing with scanii-cli
//!
//! ```bash
//! docker run -d --name scanii-cli -p 4000:4000 ghcr.io/scanii/scanii-cli:latest server
//! cargo test
//! ```
//!
//! Set `SCANII_TEST_ENDPOINT` to override the default `http://localhost:4000`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub use client::{ScaniiClient, ScaniiClientBuilder, VERSION};
pub use error::ScaniiError;
pub use models::{ScaniiAuthToken, ScaniiPendingResult, ScaniiProcessingResult};

mod client;
mod error;
mod models;
mod multipart;
