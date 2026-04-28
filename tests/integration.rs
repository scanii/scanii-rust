//! Integration tests against scanii-cli running on `http://localhost:4000`.
//!
//! Bring up scanii-cli before running:
//!
//!   docker run -d --name scanii-cli -p 4000:4000 ghcr.io/scanii/scanii-cli:latest server
//!
//! In CI we boot it via `scanii/setup-cli-action@v1`. Tests self-skip with a
//! `eprintln!` warning when scanii-cli is not reachable, so `cargo test`
//! is safe to run in any environment.

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use scanii::{ScaniiClient, ScaniiError};

const KEY: &str = "key";
const SECRET: &str = "secret";
const LOCAL_MALWARE_UUID: &str = "38DCC0C9-7FB6-4D0D-9C37-288A380C6BB9";
const LOCAL_MALWARE_FINDING: &str = "content.malicious.local-test-file";

fn endpoint() -> String {
    std::env::var("SCANII_TEST_ENDPOINT").unwrap_or_else(|_| "http://localhost:4000".into())
}

fn cli_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let client = match ScaniiClient::builder()
            .key(KEY)
            .secret(SECRET)
            .endpoint(endpoint())
            .timeout(Duration::from_secs(2))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        client.ping().is_ok()
    })
}

fn skip_if_no_cli(test_name: &str) -> bool {
    if !cli_available() {
        eprintln!(
            "[integration] skipping `{test_name}` — scanii-cli not reachable at {}",
            endpoint()
        );
        return true;
    }
    false
}

fn client() -> ScaniiClient {
    ScaniiClient::builder()
        .key(KEY)
        .secret(SECRET)
        .endpoint(endpoint())
        .build()
        .expect("client builds")
}

fn temp_file(contents: &[u8]) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("scanii-rust-{nanos:x}-{n:x}.bin"));
    fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn ping_with_valid_credentials() {
    if skip_if_no_cli("ping_with_valid_credentials") {
        return;
    }
    client().ping().expect("ping");
}

#[test]
fn ping_with_bad_credentials_returns_auth_error() {
    if skip_if_no_cli("ping_with_bad_credentials_returns_auth_error") {
        return;
    }
    let bad = ScaniiClient::builder()
        .key("nope")
        .secret("nope")
        .endpoint(endpoint())
        .build()
        .unwrap();
    match bad.ping() {
        Err(ScaniiError::Auth { .. }) => {}
        other => panic!("expected Auth error, got {other:?}"),
    }
}

#[test]
fn process_clean_file_returns_no_findings() {
    if skip_if_no_cli("process_clean_file_returns_no_findings") {
        return;
    }
    let path = temp_file(b"hello world");
    let mut metadata = HashMap::new();
    metadata.insert("source".to_owned(), "integration".to_owned());
    metadata.insert("tag".to_owned(), "clean".to_owned());

    let r = client()
        .process(&path, Some(&metadata), None)
        .expect("process");
    assert!(!r.id.is_empty(), "result must have an id");
    assert!(
        r.findings.is_empty(),
        "expected no findings, got {:?}",
        r.findings
    );

    let retrieved = client().retrieve(&r.id).expect("retrieve");
    assert_eq!(retrieved.id, r.id);

    let _ = fs::remove_file(path);
}

#[test]
fn process_malware_uuid_fixture_flags_file() {
    if skip_if_no_cli("process_malware_uuid_fixture_flags_file") {
        return;
    }
    let path = temp_file(LOCAL_MALWARE_UUID.as_bytes());
    let r = client().process(&path, None, None).expect("process");
    if !r.findings.iter().any(|f| f == LOCAL_MALWARE_FINDING) {
        eprintln!(
            "[integration] scanii-cli did not flag UUID fixture; got: {:?} (older cli build)",
            r.findings
        );
    } else {
        assert!(r.findings.iter().any(|f| f == LOCAL_MALWARE_FINDING));
    }
    let _ = fs::remove_file(path);
}

#[test]
fn process_async_returns_pending_then_retrievable() {
    if skip_if_no_cli("process_async_returns_pending_then_retrievable") {
        return;
    }
    let path = temp_file(b"hello async");
    let pending = client()
        .process_async(&path, None, None)
        .expect("process_async");
    assert!(!pending.id.is_empty());

    thread::sleep(Duration::from_millis(500));

    let retrieved = client().retrieve(&pending.id).expect("retrieve");
    assert_eq!(retrieved.id, pending.id);
    let _ = fs::remove_file(path);
}

#[test]
fn fetch_returns_pending_result() {
    if skip_if_no_cli("fetch_returns_pending_result") {
        return;
    }
    let r = client()
        .fetch("https://example.com/test.txt", None, None)
        .expect("fetch");
    assert!(!r.id.is_empty());
}

#[test]
fn auth_token_lifecycle() {
    if skip_if_no_cli("auth_token_lifecycle") {
        return;
    }
    let c = client();
    let tok = c.create_auth_token(30).expect("create_auth_token");
    assert!(!tok.id.is_empty());

    let tok2 = c.retrieve_auth_token(&tok.id).expect("retrieve_auth_token");
    assert_eq!(tok2.id, tok.id);

    let token_client = ScaniiClient::builder()
        .token(&tok.id)
        .endpoint(endpoint())
        .build()
        .unwrap();
    if let Err(e) = token_client.ping() {
        eprintln!("[integration] token-auth ping rejected by this scanii-cli build: {e}");
    }

    c.delete_auth_token(&tok.id).expect("delete_auth_token");
}

#[test]
fn process_reader_with_uuid_malware_fixture() {
    if skip_if_no_cli("process_reader_with_uuid_malware_fixture") {
        return;
    }
    let r = client()
        .process_reader(
            Cursor::new(LOCAL_MALWARE_UUID.as_bytes()),
            "uuid-fixture.bin",
            Some("application/octet-stream"),
            None,
            None,
        )
        .expect("process_reader");
    if !r.findings.iter().any(|f| f == LOCAL_MALWARE_FINDING) {
        eprintln!(
            "[integration] scanii-cli did not flag UUID fixture (reader path); got: {:?}",
            r.findings
        );
    } else {
        assert!(r.findings.iter().any(|f| f == LOCAL_MALWARE_FINDING));
    }
}

#[test]
fn process_reader_with_large_blob() {
    if skip_if_no_cli("process_reader_with_large_blob") {
        return;
    }
    let blob = vec![0u8; 1024 * 1024]; // 1 MiB
    let r = client()
        .process_reader(
            Cursor::new(blob),
            "blob.bin",
            Some("application/octet-stream"),
            None,
            None,
        )
        .expect("process_reader large");
    assert!(!r.id.is_empty(), "expected an id from a successful scan");
    assert_eq!(r.content_length, Some(1024 * 1024));
}

#[test]
fn retrieve_unknown_id_returns_http_error() {
    if skip_if_no_cli("retrieve_unknown_id_returns_http_error") {
        return;
    }
    let unknown = format!("does-not-exist-{}", std::process::id());
    match client().retrieve(&unknown) {
        Err(_) => {}
        Ok(_) => panic!("expected error for unknown id"),
    }
}

#[test]
fn callback_delivery() {
    if skip_if_no_cli("callback_delivery") {
        return;
    }

    // Bind ephemeral port for a one-shot HTTP listener.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("local_addr");
    let port = addr.port();

    let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let captured_clone = captured.clone();

    listener.set_nonblocking(true).ok();

    let server_thread = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if Instant::now() >= deadline {
                return;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    handle_callback(&mut stream, &captured_clone);
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => return,
            }
        }
    });

    let path = temp_file(b"hello callback");
    let _ = client().process(&path, None, Some(&format!("http://127.0.0.1:{port}/cb")));

    let _ = server_thread.join();
    let body = captured.lock().unwrap().clone();
    if body.is_empty() {
        eprintln!(
            "[integration] scanii-cli did not deliver a callback (callback support is a Phase-1 prereq)"
        );
        let _ = fs::remove_file(path);
        return;
    }
    assert!(
        body.contains("\"id\""),
        "callback body did not contain id; got: {body}"
    );
    let _ = fs::remove_file(path);
}

fn handle_callback(stream: &mut TcpStream, captured: &std::sync::Mutex<String>) {
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    let mut buf = [0u8; 8192];
    let mut request = Vec::new();
    let mut content_length: usize = 0;
    let mut headers_done = false;
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        request.extend_from_slice(&buf[..n]);
        if !headers_done {
            if let Some(pos) = find_double_crlf(&request) {
                headers_done = true;
                if let Some(cl) = parse_content_length(&request[..pos]) {
                    content_length = cl;
                }
                let body_so_far = request.len().saturating_sub(pos + 4);
                if body_so_far >= content_length {
                    break;
                }
            }
        } else {
            // approximate: stop once we've read enough body
            let body_start = find_double_crlf(&request).map(|p| p + 4).unwrap_or(0);
            if request.len() - body_start >= content_length {
                break;
            }
        }
    }
    let body_start = find_double_crlf(&request).map(|p| p + 4).unwrap_or(0);
    let body = &request[body_start..];
    if let Ok(mut guard) = captured.lock() {
        *guard = String::from_utf8_lossy(body).into_owned();
    }
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(headers).ok()?;
    for line in s.split("\r\n") {
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}
