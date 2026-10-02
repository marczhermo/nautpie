//! Embed `nautpie` as a library function.
//!
//! This example shows how a downstream Rust program can use `nautpie`'s
//! library surface without going through the CLI. It builds an
//! `ApiResponse::ok` envelope, which is the same shape the binary prints on
//! stdout.
//!
//! Run with: `cargo run --example embed_as_library`

use nautpie::error::ApiResponse;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The library exposes the same `ApiResponse` envelope the binary prints.
    // This is useful for embedding `nautpie` in a larger Rust program that
    // wants to serialise its own responses in the same JSON-line shape.
    let resp = ApiResponse::ok(json!({"hello": "world"}));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.reason, "OK");

    // The error envelope mirrors the success one. `ApiResponse::error`
    // accepts any status code and reason phrase.
    let err = ApiResponse::error(404, "Not Found", json!("missing"));
    assert_eq!(err.status, 404);

    // `to_line()` produces a single line of JSON with no trailing newline,
    // matching the binary's stdout contract.
    let line = resp.to_line();
    println!("{line}");
    assert!(!line.contains('\n'));

    Ok(())
}
