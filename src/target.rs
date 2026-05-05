//! Scanii regional API endpoints.

/// Scanii regional API endpoint.
///
/// Use one of the regional constructors for production, or
/// [`ScaniiTarget::from_url`] for local testing against scanii-cli:
///
/// ```
/// use scanii::ScaniiTarget;
/// let t = ScaniiTarget::from_url("http://localhost:4000");
/// ```
///
/// Note: `AUTO` (latency-based routing to `https://api.scanii.com`) is
/// intentionally absent. Explicit regional selection is required for data
/// residency compliance. Other Scanii SDKs that historically defaulted to
/// AUTO are being updated to deprecate it.
///
/// See <https://scanii.github.io/openapi/v22/>.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScaniiTarget {
    pub(crate) url: String,
}

impl ScaniiTarget {
    /// US East (N. Virginia) — `https://api-us1.scanii.com`
    pub fn us1() -> Self {
        Self::from_url("https://api-us1.scanii.com")
    }

    /// EU West (Ireland) — `https://api-eu1.scanii.com`
    pub fn eu1() -> Self {
        Self::from_url("https://api-eu1.scanii.com")
    }

    /// EU Central (Frankfurt) — `https://api-eu2.scanii.com`
    pub fn eu2() -> Self {
        Self::from_url("https://api-eu2.scanii.com")
    }

    /// Asia Pacific (Singapore) — `https://api-ap1.scanii.com`
    pub fn ap1() -> Self {
        Self::from_url("https://api-ap1.scanii.com")
    }

    /// Asia Pacific (Tokyo) — `https://api-ap2.scanii.com`
    pub fn ap2() -> Self {
        Self::from_url("https://api-ap2.scanii.com")
    }

    /// Canada (Central) — `https://api-ca1.scanii.com`
    pub fn ca1() -> Self {
        Self::from_url("https://api-ca1.scanii.com")
    }

    /// Build a [`ScaniiTarget`] from any base URL string.
    ///
    /// Use this for local testing against scanii-cli or other custom
    /// environments:
    ///
    /// ```
    /// use scanii::ScaniiTarget;
    /// let t = ScaniiTarget::from_url("http://localhost:4000");
    /// ```
    pub fn from_url(url: impl Into<String>) -> Self {
        ScaniiTarget { url: url.into() }
    }

    /// The base URL this target points at.
    pub fn url(&self) -> &str {
        &self.url
    }
}
