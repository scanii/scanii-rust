//! Quickstart example.
//!
//! Run with:
//!
//!   SCANII_KEY=...your-key... SCANII_SECRET=...your-secret... \
//!     cargo run --example quickstart -- ./path/to/file
//!
//! Or against a local scanii-cli (no real credentials needed):
//!
//!   SCANII_KEY=key SCANII_SECRET=secret SCANII_ENDPOINT=http://localhost:4000 \
//!     cargo run --example quickstart -- ./path/to/file

use std::path::Path;

use scanii::ScaniiClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("SCANII_KEY").map_err(|_| "set SCANII_KEY")?;
    let secret = std::env::var("SCANII_SECRET").map_err(|_| "set SCANII_SECRET")?;

    let mut builder = ScaniiClient::builder().key(key).secret(secret);
    if let Ok(endpoint) = std::env::var("SCANII_ENDPOINT") {
        builder = builder.endpoint(endpoint);
    }
    let client = builder.build()?;

    let file_arg = std::env::args().nth(1).ok_or("usage: quickstart <file>")?;
    let path = Path::new(&file_arg);

    let result = client.process_file(path, None, None)?;
    println!("id:       {}", result.id);
    println!("findings: {:?}", result.findings);
    if let Some(checksum) = &result.checksum {
        println!("sha1:     {checksum}");
    }
    Ok(())
}
