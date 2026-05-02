//! Hand-rolled `multipart/form-data` encoder (RFC 7578).
//!
//! ureq does not bundle a multipart encoder; this is the smallest viable
//! implementation that covers the Scanii `POST /files` payload.
//!
//! The body is split into a small **prologue** (text fields + the file part
//! header) and a small **epilogue** (closing boundary). The file content
//! itself is streamed by the caller — wrapping the prologue + reader +
//! epilogue with `std::io::Read::chain` keeps memory use independent of
//! file size.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static BOUNDARY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique multipart boundary string.
pub(crate) fn make_boundary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = BOUNDARY_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("----scanii-rust-boundary-{nanos:x}-{counter:x}")
}

/// Build the `Content-Type` header value for a request using `boundary`.
pub(crate) fn make_content_type(boundary: &str) -> String {
    format!("multipart/form-data; boundary={boundary}")
}

/// Build the bytes that go *before* the file content: text-field parts
/// followed by the file part header (boundary, Content-Disposition,
/// Content-Type, blank line).
pub(crate) fn build_prologue(
    boundary: &str,
    filename: &str,
    content_type: &str,
    text_fields: &HashMap<String, String>,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();

    for (name, value) in text_fields {
        write_text_part(&mut out, boundary, name, value);
    }

    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"");
    out.extend_from_slice(filename.as_bytes());
    out.extend_from_slice(b"\"\r\n");
    out.extend_from_slice(b"Content-Type: ");
    out.extend_from_slice(content_type.as_bytes());
    out.extend_from_slice(b"\r\n\r\n");

    out
}

/// Build the bytes that go *after* the file content: the trailing CRLF
/// that closes the file part, then the closing boundary.
pub(crate) fn build_epilogue(boundary: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"--\r\n");
    out
}

fn write_text_part(out: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(b"Content-Disposition: form-data; name=\"");
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(b"\"\r\n");
    out.extend_from_slice(b"Content-Type: text/plain; charset=UTF-8\r\n\r\n");
    out.extend_from_slice(value.as_bytes());
    out.extend_from_slice(b"\r\n");
}

/// Build a complete multipart/form-data body containing only text fields —
/// no file part. Use when submitting a URL via `process_from_url`.
pub(crate) fn build_text_only_body(
    boundary: &str,
    text_fields: &HashMap<String, String>,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for (name, value) in text_fields {
        write_text_part(&mut out, boundary, name, value);
    }
    // Closing boundary. No leading \r\n here — write_text_part already appended
    // \r\n after the value, and the binary-file epilogue's leading \r\n is only
    // needed to terminate raw file bytes that carry no trailing CRLF of their own.
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"--\r\n");
    out
}

/// Best-effort content-type lookup by extension. Falls back to
/// `application/octet-stream`. The Scanii API does not require an accurate
/// content-type on the multipart part — the server inspects the bytes — so a
/// short table is sufficient.
pub(crate) fn guess_content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some("txt") => "text/plain",
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz") => "application/gzip",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Assemble the same body a streaming request would: prologue + content + epilogue.
    fn assemble(prologue: &[u8], content: &[u8], epilogue: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(prologue.len() + content.len() + epilogue.len());
        out.extend_from_slice(prologue);
        out.extend_from_slice(content);
        out.extend_from_slice(epilogue);
        out
    }

    #[test]
    fn encode_emits_well_formed_body() {
        let boundary = "test-boundary-123";
        let mut fields = HashMap::new();
        fields.insert("metadata[source]".into(), "unit".into());

        let prologue = build_prologue(boundary, "hello.txt", "text/plain", &fields);
        let epilogue = build_epilogue(boundary);
        let content = b"hello world";

        let body = assemble(&prologue, content, &epilogue);
        let body_str = String::from_utf8_lossy(&body);

        assert!(body_str.contains("name=\"metadata[source]\""));
        assert!(body_str.contains("name=\"file\"; filename=\"hello.txt\""));
        assert!(body_str.contains("hello world"));
        assert!(body_str.ends_with("--\r\n"));
        assert!(body_str.contains(&format!("--{boundary}--\r\n")));
    }

    #[test]
    fn boundaries_are_unique() {
        let a = make_boundary();
        let b = make_boundary();
        assert_ne!(a, b);
    }

    #[test]
    fn make_content_type_includes_boundary() {
        let ct = make_content_type("xyz");
        assert_eq!(ct, "multipart/form-data; boundary=xyz");
    }

    #[test]
    fn guess_content_type_known_extensions() {
        assert_eq!(guess_content_type(Path::new("a.txt")), "text/plain");
        assert_eq!(guess_content_type(Path::new("a.PDF")), "application/pdf");
        assert_eq!(
            guess_content_type(Path::new("nope")),
            "application/octet-stream"
        );
    }
}
