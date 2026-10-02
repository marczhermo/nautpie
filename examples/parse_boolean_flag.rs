//! Parse boolean CLI flags the way `nautpie` does.
//!
//! Demonstrates the lenient boolean parser used by `nautpie` for flags like
//! `--bypass_and_start`. Accepts `true`/`false`, `yes`/`no`, `1`/`0`, and
//! the empty string (which counts as `false`). Anything else is an error.
//!
//! Run with: `cargo run --example parse_boolean_flag`

use nautpie::boolean::check_boolean;
use nautpie::error::Error;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The parser accepts many synonyms. These all return `Ok(true)`:
    for input in ["true", "TRUE", "yes", "YES", "1"] {
        let v = check_boolean(Some(input))?;
        assert!(v);
        println!("{input:?} -> {v}");
    }

    // And these all return `Ok(false)`:
    for input in [
        Some("false"),
        Some("FALSE"),
        Some("no"),
        Some("0"),
        Some(""),
        None,
    ] {
        let v = check_boolean(input)?;
        assert!(!v);
        println!("{input:?} -> {v}");
    }

    // Unknown values produce a `Generic` error (not a panic).
    match check_boolean(Some("maybe")) {
        Ok(v) => panic!("expected error, got Ok({v})"),
        Err(Error::Generic(msg)) => {
            println!("got expected error: {msg}");
        }
        Err(other) => panic!("unexpected error variant: {other:?}"),
    }

    Ok(())
}
