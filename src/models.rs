use serde::Deserialize;
use std::collections::HashMap;

/// Result of a synchronous file scan returned by [`crate::ScaniiClient::process`]
/// and [`crate::ScaniiClient::retrieve`].
///
/// `findings` is always populated — empty when the content is clean.
///
/// See <https://scanii.github.io/openapi/v22/>.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScaniiProcessingResult {
    /// Resource id assigned by the API.
    pub id: String,

    /// Detection findings, e.g. `content.malicious.local-test-file`. Empty when clean.
    #[serde(default)]
    pub findings: Vec<String>,

    /// SHA-1 checksum of the uploaded content, when reported.
    #[serde(default)]
    pub checksum: Option<String>,

    /// Size of the uploaded content in bytes, when reported.
    #[serde(default)]
    pub content_length: Option<u64>,

    /// MIME type detected by the API, when reported.
    #[serde(default)]
    pub content_type: Option<String>,

    /// Caller-supplied metadata echoed back by the API.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Server-side creation timestamp (ISO 8601), when reported.
    #[serde(default)]
    pub creation_date: Option<String>,

    /// Server-supplied error message — populated only when the API set one.
    #[serde(default)]
    pub error: Option<String>,

    /// `X-Scanii-Request-Id` response header, populated by the client after deserialization.
    #[serde(skip)]
    pub request_id: Option<String>,

    /// `X-Scanii-Host-Id` response header, populated by the client after deserialization.
    #[serde(skip)]
    pub host_id: Option<String>,

    /// `Location` response header, populated by the client after deserialization.
    #[serde(skip)]
    pub resource_location: Option<String>,
}

/// Result of an asynchronous scan submission returned by
/// [`crate::ScaniiClient::process_async`] and [`crate::ScaniiClient::fetch`].
///
/// The actual scan result is fetched later via [`crate::ScaniiClient::retrieve`]
/// or delivered to the supplied `callback` URL.
///
/// See <https://scanii.github.io/openapi/v22/>.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScaniiPendingResult {
    /// Resource id assigned by the API; pass to `retrieve` to read the result.
    pub id: String,

    /// `X-Scanii-Request-Id` response header, populated by the client.
    #[serde(skip)]
    pub request_id: Option<String>,

    /// `X-Scanii-Host-Id` response header, populated by the client.
    #[serde(skip)]
    pub host_id: Option<String>,

    /// `Location` response header, populated by the client.
    #[serde(skip)]
    pub resource_location: Option<String>,
}

/// Short-lived auth token returned by [`crate::ScaniiClient::create_auth_token`]
/// and [`crate::ScaniiClient::retrieve_auth_token`].
///
/// Pass `id` to [`crate::ScaniiClientBuilder::token`] to authenticate using the
/// token instead of API key + secret.
///
/// See <https://scanii.github.io/openapi/v22/>.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScaniiAuthToken {
    /// Token id — opaque string used as the auth credential.
    pub id: String,

    /// Server-side creation timestamp (ISO 8601), when reported.
    #[serde(default)]
    pub creation_date: Option<String>,

    /// Token expiry timestamp (ISO 8601), when reported.
    #[serde(default)]
    pub expiration_date: Option<String>,

    /// `X-Scanii-Request-Id` response header, populated by the client.
    #[serde(skip)]
    pub request_id: Option<String>,

    /// `X-Scanii-Host-Id` response header, populated by the client.
    #[serde(skip)]
    pub host_id: Option<String>,

    /// `Location` response header, populated by the client.
    #[serde(skip)]
    pub resource_location: Option<String>,
}
