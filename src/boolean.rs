//! PHP `CheckHelper::checkBoolean` parity.
//!
//! Mirrors the original lenient coercion: `true`/`yes`/`1` → true;
//! `false`/`no`/`0` → false; empty / null / None → false; anything else → error.

use crate::error::Error;

/// Convert a CLI string value into a boolean, matching PHP semantics.
///
/// Accepts an `Option<&str>` so callers can pass through `Option::None` for
/// flags that the user did not supply.
pub fn check_boolean(value: Option<&str>) -> Result<bool, Error> {
    match value.map(str::trim).map(str::to_ascii_lowercase) {
        None => Ok(false),
        Some(v) if v.is_empty() => Ok(false),
        Some(v) if matches!(v.as_str(), "false" | "no" | "0") => Ok(false),
        Some(v) if matches!(v.as_str(), "true" | "yes" | "1") => Ok(true),
        Some(v) => Err(Error::Generic(format!(
            "[Boolean] Value not accepted as boolean: {v}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn false_variants() {
        for v in [
            Some(""),
            Some("0"),
            Some("no"),
            Some("false"),
            Some("FALSE"),
            Some("No"),
            None,
        ] {
            assert_eq!(check_boolean(v), Ok(false), "input: {v:?}");
        }
    }

    #[test]
    fn true_variants() {
        for v in [
            Some("1"),
            Some("yes"),
            Some("true"),
            Some("YES"),
            Some("True"),
        ] {
            assert_eq!(check_boolean(v), Ok(true), "input: {v:?}");
        }
    }

    #[test]
    fn invalid_variant_errors() {
        assert!(check_boolean(Some("maybe")).is_err());
        assert!(check_boolean(Some("2")).is_err());
    }
}
