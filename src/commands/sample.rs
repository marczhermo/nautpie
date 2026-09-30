//! The `sampleSuccess` / `sampleFail` smoke actions. Mirror the PHP
//! `CommandsHelper` trait methods exactly.

use serde_json::json;

use crate::error::{ApiResponse, Error};

#[allow(dead_code)]
pub const SUCCESS_MESSAGE: &str = "[Action:Success] Response successful.";
#[allow(dead_code)]
pub const FAIL_MESSAGE: &str = "[Action:Fail] Has failed.";

/// PHP `CommandsHelper::doSampleSuccess` — returns the success message
/// string (json-encoded by the dispatcher).
pub fn sample_success() -> ApiResponse {
    ApiResponse::ok(json!(SUCCESS_MESSAGE))
}

/// PHP `CommandsHelper::doSampleFail` — throws an exception that the
/// dispatcher turns into a non-zero exit.
pub fn sample_fail() -> Result<ApiResponse, Error> {
    Err(Error::Generic(FAIL_MESSAGE.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_message_round_trip() {
        let r = sample_success();
        assert_eq!(r.status, 200);
        assert_eq!(r.reason, "OK");
        assert_eq!(r.body, json!(SUCCESS_MESSAGE));
    }

    #[test]
    fn fail_returns_error() {
        let err = sample_fail().unwrap_err();
        assert!(matches!(err, Error::Generic(ref m) if m == FAIL_MESSAGE));
    }
}
