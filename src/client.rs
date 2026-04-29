use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::Duration;

use ureq::{Agent, AgentBuilder, Request, Response};

use crate::error::ScaniiError;
use crate::models::{ScaniiAuthToken, ScaniiPendingResult, ScaniiProcessingResult};
use crate::multipart;

const DEFAULT_ENDPOINT: &str = "https://api.scanii.com";
const API_VERSION_PATH: &str = "/v2.2";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Crate version, inlined at compile time from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Synchronous client for the Scanii REST API v2.2.
///
/// Construct via [`ScaniiClient::builder`]. The client is `Send + Sync` and
/// can be safely shared across threads.
///
/// Per SDK Principle 3 the client is integration-only: it does not retry,
/// batch, or paginate. Each public method maps to exactly one HTTP request.
///
/// See <https://scanii.github.io/openapi/v22/>.
///
/// # Example
///
/// ```no_run
/// # use scanii::ScaniiClient;
/// # fn main() -> Result<(), scanii::ScaniiError> {
/// let client = ScaniiClient::builder()
///     .key("your-key")
///     .secret("your-secret")
///     .build()?;
/// client.ping()?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct ScaniiClient {
    agent: Agent,
    base_url: String,
    auth_header: String,
    user_agent: String,
}

/// Builder for [`ScaniiClient`].
///
/// Set either `key` + `secret` (HTTP Basic Auth), or `token` (auth-token
/// authentication). Mixing the two is a configuration error.
#[derive(Debug, Default, Clone)]
pub struct ScaniiClientBuilder {
    key: Option<String>,
    secret: Option<String>,
    token: Option<String>,
    endpoint: Option<String>,
    user_agent: Option<String>,
    timeout: Option<Duration>,
}

struct ResponseHeaders {
    request_id: Option<String>,
    host_id: Option<String>,
    location: Option<String>,
}

impl ScaniiClient {
    /// Start building a client.
    pub fn builder() -> ScaniiClientBuilder {
        ScaniiClientBuilder::default()
    }

    /// Submit content from any [`Read`] source for synchronous scanning.
    ///
    /// The body is streamed — memory use is independent of content length.
    ///
    /// `filename` goes verbatim into the multipart `Content-Disposition`
    /// header; `content_type` defaults to `application/octet-stream` when
    /// `None`.
    ///
    /// For file-on-disk uploads, prefer the [`Self::process_file`] convenience.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /files`.
    pub fn process<R: Read>(
        &self,
        reader: R,
        filename: &str,
        content_type: Option<&str>,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiProcessingResult, ScaniiError> {
        let ct = content_type.unwrap_or("application/octet-stream");
        let response =
            self.post_multipart_streaming("/files", reader, filename, ct, metadata, callback)?;
        let response = require_status(response, 201)?;
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiProcessingResult = parse_json(&body)?;
        attach_processing_headers(&mut result, headers);
        Ok(result)
    }

    /// Submit a file from disk for synchronous scanning.
    ///
    /// Opens the file with `BufReader`, derives the filename from the path,
    /// and infers the content type by extension. Delegates to [`Self::process`].
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /files`.
    pub fn process_file(
        &self,
        path: &Path,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiProcessingResult, ScaniiError> {
        let filename = path_filename(path);
        let content_type = multipart::guess_content_type(path);
        let reader = BufReader::new(File::open(path)?);
        self.process(reader, &filename, Some(content_type), metadata, callback)
    }

    /// Submit content from any [`Read`] source for server-side asynchronous
    /// scanning. The body is streamed — memory use is independent of content
    /// length. Returns a pending id; the final result is delivered to
    /// `callback` (when supplied) or fetched via [`Self::retrieve`].
    ///
    /// For file-on-disk uploads, prefer the [`Self::process_async_file`] convenience.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /files/async`.
    pub fn process_async<R: Read>(
        &self,
        reader: R,
        filename: &str,
        content_type: Option<&str>,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiPendingResult, ScaniiError> {
        let ct = content_type.unwrap_or("application/octet-stream");
        let response = self.post_multipart_streaming(
            "/files/async",
            reader,
            filename,
            ct,
            metadata,
            callback,
        )?;
        let response = require_status(response, 202)?;
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiPendingResult = parse_json(&body)?;
        attach_pending_headers(&mut result, headers);
        Ok(result)
    }

    /// Submit a file from disk for server-side asynchronous scanning.
    ///
    /// Opens the file with `BufReader` and delegates to [`Self::process_async`].
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /files/async`.
    pub fn process_async_file(
        &self,
        path: &Path,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiPendingResult, ScaniiError> {
        let filename = path_filename(path);
        let content_type = multipart::guess_content_type(path);
        let reader = BufReader::new(File::open(path)?);
        self.process_async(reader, &filename, Some(content_type), metadata, callback)
    }

    /// Deprecated: use [`Self::process`] (stream-based) instead.
    #[deprecated(
        since = "1.1.0",
        note = "use `process` (stream-based); will be removed in a future major version"
    )]
    pub fn process_reader<R: Read>(
        &self,
        reader: R,
        filename: &str,
        content_type: Option<&str>,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiProcessingResult, ScaniiError> {
        self.process(reader, filename, content_type, metadata, callback)
    }

    /// Deprecated: use [`Self::process_file`] (path-based convenience) instead.
    #[deprecated(
        since = "1.1.0",
        note = "use `process_file`; will be removed in a future major version"
    )]
    pub fn process_path(
        &self,
        path: &Path,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiProcessingResult, ScaniiError> {
        self.process_file(path, metadata, callback)
    }

    /// Deprecated: use [`Self::process_async`] (stream-based) instead.
    #[deprecated(
        since = "1.1.0",
        note = "use `process_async` (stream-based); will be removed in a future major version"
    )]
    pub fn process_async_reader<R: Read>(
        &self,
        reader: R,
        filename: &str,
        content_type: Option<&str>,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiPendingResult, ScaniiError> {
        self.process_async(reader, filename, content_type, metadata, callback)
    }

    /// Deprecated: use [`Self::process_async_file`] (path-based convenience) instead.
    #[deprecated(
        since = "1.1.0",
        note = "use `process_async_file`; will be removed in a future major version"
    )]
    pub fn process_async_path(
        &self,
        path: &Path,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiPendingResult, ScaniiError> {
        self.process_async_file(path, metadata, callback)
    }

    /// Ask Scanii to download a remote URL and scan it asynchronously.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /files/fetch`.
    pub fn fetch(
        &self,
        location: &str,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<ScaniiPendingResult, ScaniiError> {
        if location.is_empty() {
            return Err(ScaniiError::Config("location must not be empty".into()));
        }

        let mut owned: Vec<(String, String)> = vec![("location".into(), location.to_owned())];
        if let Some(cb) = callback {
            owned.push(("callback".into(), cb.to_owned()));
        }
        if let Some(m) = metadata {
            for (k, v) in m {
                owned.push((format!("metadata[{k}]"), v.clone()));
            }
        }
        let pairs: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let response = self.request("POST", "/files/fetch").send_form(&pairs)?;
        let response = require_status(response, 202)?;
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiPendingResult = parse_json(&body)?;
        attach_pending_headers(&mut result, headers);
        Ok(result)
    }

    /// Retrieve a previously submitted scan result by id.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `GET /files/{id}`.
    pub fn retrieve(&self, id: &str) -> Result<ScaniiProcessingResult, ScaniiError> {
        if id.is_empty() {
            return Err(ScaniiError::Config("id must not be empty".into()));
        }
        let path = format!("/files/{}", url_encode(id));
        let response = self.request("GET", &path).call()?;
        let response = require_status(response, 200)?;
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiProcessingResult = parse_json(&body)?;
        attach_processing_headers(&mut result, headers);
        Ok(result)
    }

    /// Verify that the configured credentials reach the API.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `GET /ping`.
    pub fn ping(&self) -> Result<(), ScaniiError> {
        let response = self.request("GET", "/ping").call()?;
        let _ = require_status(response, 200)?;
        Ok(())
    }

    /// Mint a short-lived auth token. `timeout_seconds` must be positive.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `POST /auth/tokens`.
    pub fn create_auth_token(&self, timeout_seconds: u64) -> Result<ScaniiAuthToken, ScaniiError> {
        if timeout_seconds == 0 {
            return Err(ScaniiError::Config(
                "timeout_seconds must be positive".into(),
            ));
        }
        let timeout_str = timeout_seconds.to_string();
        let response = self
            .request("POST", "/auth/tokens")
            .send_form(&[("timeout", timeout_str.as_str())])?;

        let status = response.status();
        if status != 200 && status != 201 {
            return Err(error_from_response(response));
        }
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiAuthToken = parse_json(&body)?;
        attach_auth_token_headers(&mut result, headers);
        Ok(result)
    }

    /// Inspect a previously created auth token.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `GET /auth/tokens/{id}`.
    pub fn retrieve_auth_token(&self, id: &str) -> Result<ScaniiAuthToken, ScaniiError> {
        if id.is_empty() {
            return Err(ScaniiError::Config("id must not be empty".into()));
        }
        let path = format!("/auth/tokens/{}", url_encode(id));
        let response = self.request("GET", &path).call()?;
        let response = require_status(response, 200)?;
        let headers = capture_headers(&response);
        let body = response_to_string(response)?;
        let mut result: ScaniiAuthToken = parse_json(&body)?;
        attach_auth_token_headers(&mut result, headers);
        Ok(result)
    }

    /// Revoke an auth token.
    ///
    /// See <https://scanii.github.io/openapi/v22/> — `DELETE /auth/tokens/{id}`.
    pub fn delete_auth_token(&self, id: &str) -> Result<(), ScaniiError> {
        if id.is_empty() {
            return Err(ScaniiError::Config("id must not be empty".into()));
        }
        let path = format!("/auth/tokens/{}", url_encode(id));
        let response = self.request("DELETE", &path).call()?;
        let _ = require_status(response, 204)?;
        Ok(())
    }

    fn request(&self, method: &str, path: &str) -> Request {
        let url = format!("{}{}", self.base_url, path);
        self.agent
            .request(method, &url)
            .set("Authorization", &self.auth_header)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/json")
    }

    fn post_multipart_streaming<R: Read>(
        &self,
        path: &str,
        reader: R,
        filename: &str,
        content_type: &str,
        metadata: Option<&HashMap<String, String>>,
        callback: Option<&str>,
    ) -> Result<Response, ScaniiError> {
        let boundary = multipart::make_boundary();
        let ct = multipart::make_content_type(&boundary);

        let mut fields: HashMap<String, String> = HashMap::new();
        if let Some(m) = metadata {
            for (k, v) in m {
                fields.insert(format!("metadata[{k}]"), v.clone());
            }
        }
        if let Some(cb) = callback {
            fields.insert("callback".into(), cb.to_owned());
        }

        let prologue = multipart::build_prologue(&boundary, filename, content_type, &fields);
        let epilogue = multipart::build_epilogue(&boundary);

        let body = std::io::Cursor::new(prologue)
            .chain(reader)
            .chain(std::io::Cursor::new(epilogue));

        Ok(self
            .request("POST", path)
            .set("Content-Type", &ct)
            .send(body)?)
    }
}

fn path_filename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_owned())
}

impl ScaniiClientBuilder {
    /// Set the API key. Cannot contain a colon.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the API secret. Pair with [`Self::key`].
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Authenticate with a previously minted auth-token id instead of `key` + `secret`.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Override the API endpoint. Defaults to `https://api.scanii.com`.
    /// Use regional hosts (`https://api-eu1.scanii.com`, etc.) or
    /// `http://localhost:4000` for scanii-cli local testing.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Optional user-agent fragment prepended to the SDK's default.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Override the request timeout. Default is 60 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Validate and assemble the client.
    pub fn build(self) -> Result<ScaniiClient, ScaniiError> {
        let auth_header = if let Some(token) = self.token.as_deref() {
            if self.key.is_some() || self.secret.is_some() {
                return Err(ScaniiError::Config(
                    "supply either token or key+secret, not both".into(),
                ));
            }
            if token.is_empty() {
                return Err(ScaniiError::Config("token must not be empty".into()));
            }
            format!("Basic {}", base64_encode(&format!("{token}:")))
        } else {
            let key = self.key.as_deref().ok_or_else(|| {
                ScaniiError::Config("key must be set (or use token for auth-token mode)".into())
            })?;
            if key.is_empty() {
                return Err(ScaniiError::Config("key must not be empty".into()));
            }
            if key.contains(':') {
                return Err(ScaniiError::Config("key must not contain a colon".into()));
            }
            let secret = self.secret.as_deref().ok_or_else(|| {
                ScaniiError::Config("secret must be set when using key auth".into())
            })?;
            format!("Basic {}", base64_encode(&format!("{key}:{secret}")))
        };

        let endpoint = self.endpoint.unwrap_or_else(|| DEFAULT_ENDPOINT.to_owned());
        let endpoint = endpoint.trim_end_matches('/').to_owned();
        let base_url = format!("{endpoint}{API_VERSION_PATH}");

        let default_ua = format!("scanii-rust/{VERSION}");
        let user_agent = match self.user_agent {
            Some(prefix) if !prefix.is_empty() => format!("{prefix} {default_ua}"),
            _ => default_ua,
        };

        let timeout = self
            .timeout
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let agent = AgentBuilder::new().timeout(timeout).build();

        Ok(ScaniiClient {
            agent,
            base_url,
            auth_header,
            user_agent,
        })
    }
}

fn require_status(response: Response, expected: u16) -> Result<Response, ScaniiError> {
    if response.status() == expected {
        Ok(response)
    } else {
        Err(error_from_response(response))
    }
}

fn error_from_response(response: Response) -> ScaniiError {
    let status = response.status();
    let request_id = response.header("x-scanii-request-id").map(|s| s.to_owned());
    let retry_after = response
        .header("retry-after")
        .and_then(|s| s.parse::<u64>().ok());
    let body = response.into_string().unwrap_or_default();
    let message = parse_error_message(&body).unwrap_or_else(|| format!("HTTP {status}"));

    match status {
        401 | 403 => ScaniiError::Auth {
            message,
            request_id,
        },
        429 => ScaniiError::RateLimit {
            retry_after,
            message,
            request_id,
        },
        _ => ScaniiError::Http {
            status,
            message,
            request_id,
        },
    }
}

fn parse_error_message(body: &str) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(serde_json::Value::String(err)) = map.get("error") {
            return Some(err.clone());
        }
    }
    Some(body.to_owned())
}

fn capture_headers(response: &Response) -> ResponseHeaders {
    ResponseHeaders {
        request_id: response.header("x-scanii-request-id").map(|s| s.to_owned()),
        host_id: response.header("x-scanii-host-id").map(|s| s.to_owned()),
        location: response.header("location").map(|s| s.to_owned()),
    }
}

fn attach_processing_headers(result: &mut ScaniiProcessingResult, h: ResponseHeaders) {
    result.request_id = h.request_id;
    result.host_id = h.host_id;
    result.resource_location = h.location;
}

fn attach_pending_headers(result: &mut ScaniiPendingResult, h: ResponseHeaders) {
    result.request_id = h.request_id;
    result.host_id = h.host_id;
    result.resource_location = h.location;
}

fn attach_auth_token_headers(result: &mut ScaniiAuthToken, h: ResponseHeaders) {
    result.request_id = h.request_id;
    result.host_id = h.host_id;
    result.resource_location = h.location;
}

fn response_to_string(response: Response) -> Result<String, ScaniiError> {
    response
        .into_string()
        .map_err(|e| ScaniiError::Transport(e.to_string()))
}

fn parse_json<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ScaniiError> {
    serde_json::from_str(body).map_err(ScaniiError::from)
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    out
}

/// Minimal stdlib-only base64 encoder. Used once per client (auth header).
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n =
            (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8) | u32::from(bytes[i + 2]);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let remaining = bytes.len() - i;
    if remaining == 1 {
        let n = u32::from(bytes[i]) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if remaining == 2 {
        let n = (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_rejects_empty_key() {
        let err = ScaniiClient::builder()
            .key("")
            .secret("s")
            .build()
            .unwrap_err();
        assert!(matches!(err, ScaniiError::Config(_)), "got {err:?}");
    }

    #[test]
    fn builder_rejects_colon_in_key() {
        let err = ScaniiClient::builder()
            .key("a:b")
            .secret("s")
            .build()
            .unwrap_err();
        assert!(matches!(err, ScaniiError::Config(_)));
    }

    #[test]
    fn builder_rejects_missing_secret() {
        let err = ScaniiClient::builder().key("k").build().unwrap_err();
        assert!(matches!(err, ScaniiError::Config(_)));
    }

    #[test]
    fn builder_rejects_token_plus_key() {
        let err = ScaniiClient::builder()
            .token("t")
            .key("k")
            .secret("s")
            .build()
            .unwrap_err();
        assert!(matches!(err, ScaniiError::Config(_)));
    }

    #[test]
    fn builder_token_only_succeeds() {
        let _ = ScaniiClient::builder().token("tok").build().unwrap();
    }

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foob"), "Zm9vYg==");
        assert_eq!(base64_encode("fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode("key:secret"), "a2V5OnNlY3JldA==");
    }

    #[test]
    fn url_encode_basic() {
        assert_eq!(url_encode("abc"), "abc");
        assert_eq!(url_encode("a/b"), "a%2Fb");
        assert_eq!(url_encode("a b"), "a%20b");
    }

    #[test]
    fn version_constant_matches_cargo_pkg_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    // process_file and process must produce byte-identical multipart bodies
    // for the same content. They go through the same post_multipart_streaming
    // helper, so this asserts that the prologue + reader + epilogue assembly
    // matches whether the file is opened via File::open or supplied as a Cursor.
    #[test]
    fn path_and_reader_paths_produce_equivalent_bodies() {
        use std::io::Read as _;

        let content = b"hello world from a file";
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "scanii-rust-equiv-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, content).expect("write fixture");

        let boundary = "fixed-boundary-for-test";
        let mut fields = HashMap::new();
        fields.insert("metadata[k]".into(), "v".into());

        let prologue =
            crate::multipart::build_prologue(boundary, "fixture.txt", "text/plain", &fields);
        let epilogue = crate::multipart::build_epilogue(boundary);

        // Reader-source body: stream over Cursor<&[u8]>.
        let mut reader_body = prologue.clone();
        std::io::Cursor::new(content)
            .read_to_end(&mut reader_body)
            .expect("read cursor");
        reader_body.extend_from_slice(&epilogue);

        // Path-source body: stream over File.
        let mut path_body = prologue.clone();
        std::fs::File::open(&path)
            .expect("open fixture")
            .read_to_end(&mut path_body)
            .expect("read file");
        path_body.extend_from_slice(&epilogue);

        assert_eq!(reader_body, path_body);

        let _ = std::fs::remove_file(&path);
    }
}
