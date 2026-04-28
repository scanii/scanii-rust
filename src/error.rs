use std::fmt;

/// Errors returned by [`crate::ScaniiClient`].
///
/// HTTP-status-bearing variants ([`ScaniiError::Auth`], [`ScaniiError::RateLimit`],
/// [`ScaniiError::Http`]) carry the API-supplied error message and optional
/// `X-Scanii-Request-Id` for support handoffs.
///
/// Per SDK Principle 3 (integration-only) the SDK does not retry on the
/// caller's behalf — handling backoff is the caller's responsibility.
///
/// See <https://scanii.github.io/openapi/v22/>.
#[derive(Debug)]
pub enum ScaniiError {
    /// Configuration error in the client builder (missing key, invalid endpoint, etc.).
    Config(String),
    /// HTTP 401 / 403 — the credentials were rejected by the API.
    Auth {
        /// Error message returned by the API (or a generic `HTTP {status}`).
        message: String,
        /// `X-Scanii-Request-Id` response header, when present.
        request_id: Option<String>,
    },
    /// HTTP 429 — rate-limited. `retry_after` is the value of the `Retry-After`
    /// header in seconds, when the server provided one.
    RateLimit {
        /// Value of the `Retry-After` response header, in seconds.
        retry_after: Option<u64>,
        /// Error message returned by the API.
        message: String,
        /// `X-Scanii-Request-Id` response header, when present.
        request_id: Option<String>,
    },
    /// Other non-success HTTP status (4xx / 5xx).
    Http {
        /// HTTP status code returned by the API.
        status: u16,
        /// Error message returned by the API.
        message: String,
        /// `X-Scanii-Request-Id` response header, when present.
        request_id: Option<String>,
    },
    /// Transport-level failure (DNS, TLS, connection, timeout).
    Transport(String),
    /// Failed to deserialize the API response.
    Serde(String),
    /// Local I/O error — typically while reading a file to upload.
    Io(std::io::Error),
}

impl fmt::Display for ScaniiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaniiError::Config(m) => write!(f, "configuration error: {m}"),
            ScaniiError::Auth { message, .. } => write!(f, "authentication failed: {message}"),
            ScaniiError::RateLimit {
                retry_after,
                message,
                ..
            } => match retry_after {
                Some(s) => write!(f, "rate limited (retry after {s}s): {message}"),
                None => write!(f, "rate limited: {message}"),
            },
            ScaniiError::Http {
                status, message, ..
            } => write!(f, "HTTP {status}: {message}"),
            ScaniiError::Transport(m) => write!(f, "transport error: {m}"),
            ScaniiError::Serde(m) => write!(f, "deserialization error: {m}"),
            ScaniiError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for ScaniiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScaniiError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ScaniiError {
    fn from(e: std::io::Error) -> Self {
        ScaniiError::Io(e)
    }
}

impl From<serde_json::Error> for ScaniiError {
    fn from(e: serde_json::Error) -> Self {
        ScaniiError::Serde(e.to_string())
    }
}

impl From<ureq::Error> for ScaniiError {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Status(status, response) => {
                let request_id = response.header("x-scanii-request-id").map(|s| s.to_owned());
                let retry_after = response
                    .header("retry-after")
                    .and_then(|s| s.parse::<u64>().ok());
                let body = response.into_string().unwrap_or_default();
                let message =
                    parse_error_message(&body).unwrap_or_else(|| format!("HTTP {status}"));

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
            ureq::Error::Transport(t) => ScaniiError::Transport(t.to_string()),
        }
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
