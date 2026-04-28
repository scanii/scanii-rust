//! Hand-rolled `multipart/form-data` encoder (RFC 7578).
//!
//! ureq does not bundle a multipart encoder; this is the smallest viable
//! implementation that covers the Scanii `POST /files` payload. Adds zero
//! external dependencies.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ScaniiError;

static BOUNDARY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Encode the multipart body and return `(bytes, content_type)`.
///
/// `text_fields` are emitted before the binary file part. `file_path` is read
/// in full into memory — adequate for typical Scanii payloads (the API rejects
/// > 50 MiB regardless).
pub(crate) fn encode(
    file_path: &Path,
    text_fields: &HashMap<String, String>,
) -> Result<(Vec<u8>, String), ScaniiError> {
    let boundary = make_boundary();
    let content_type = format!("multipart/form-data; boundary={boundary}");

    let mut body: Vec<u8> = Vec::new();

    for (name, value) in text_fields {
        write_text_part(&mut body, &boundary, name, value);
    }

    let filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_owned());
    let file_content_type = guess_content_type(file_path);

    let mut file = File::open(file_path)?;
    let mut file_bytes = Vec::new();
    file.read_to_end(&mut file_bytes)?;

    write_binary_part(
        &mut body,
        &boundary,
        "file",
        &filename,
        file_content_type,
        &file_bytes,
    );

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");

    Ok((body, content_type))
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

fn write_binary_part(
    out: &mut Vec<u8>,
    boundary: &str,
    field_name: &str,
    filename: &str,
    content_type: &str,
    bytes: &[u8],
) {
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(b"Content-Disposition: form-data; name=\"");
    out.extend_from_slice(field_name.as_bytes());
    out.extend_from_slice(b"\"; filename=\"");
    out.extend_from_slice(filename.as_bytes());
    out.extend_from_slice(b"\"\r\n");
    out.extend_from_slice(b"Content-Type: ");
    out.extend_from_slice(content_type.as_bytes());
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n");
}

fn make_boundary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = BOUNDARY_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("----scanii-rust-boundary-{nanos:x}-{counter:x}")
}

/// Best-effort content-type lookup by extension. Falls back to
/// `application/octet-stream`. The Scanii API does not require an accurate
/// content-type on the multipart part — the server inspects the bytes — so a
/// short table is sufficient.
fn guess_content_type(path: &Path) -> &'static str {
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
    use std::io::Write;

    #[test]
    fn encode_emits_well_formed_body() {
        let dir = std::env::temp_dir();
        let path = dir.join("scanii-rust-mp-test.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello world").unwrap();

        let mut fields = HashMap::new();
        fields.insert("metadata[source]".into(), "unit".into());

        let (body, ct) = encode(&path, &fields).unwrap();
        assert!(ct.starts_with("multipart/form-data; boundary="));

        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.contains("name=\"metadata[source]\""));
        assert!(body_str.contains("name=\"file\"; filename=\"scanii-rust-mp-test.txt\""));
        assert!(body_str.contains("hello world"));
        assert!(body_str.ends_with("--\r\n"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn boundaries_are_unique() {
        let a = make_boundary();
        let b = make_boundary();
        assert_ne!(a, b);
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
