//! Top-level error type and the JSON response envelope that the binary prints
//! on stdout. The shape mirrors the PHP version: `{"status", "reason", "body"}`,
//! where `body` may itself be a string, object, or array.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
/// The single error type returned by every fallible function in this crate.
///
/// Mirrors the PHP original's `[Prefix]`-tagged error messages and uses
/// `thiserror` to generate `Display` and `std::error::Error` impls.
pub enum Error {
    /// A required environment variable was unset or empty.
    /// The inner `String` is the variable's name.
    #[error("[Required:ENV] {0} is missing.")]
    MissingEnv(String),

    /// A required CLI option was not supplied.
    /// The inner `String` is the option's name (e.g. `"stack"`).
    #[error("[Required:Option] {0} is missing.")]
    MissingOption(String),

    /// The user invoked a subcommand with no recognisable action.
    #[error("[Missing] Action or End Point.")]
    MissingAction,

    /// The API endpoint environment variable (e.g. `NAUT_ENDPOINT`) is unset.
    #[error("[Missing] End Point is not configured.")]
    MissingEndpoint,

    /// A polling operation exceeded its deadline.
    /// The inner `String` is a human-readable description of what timed out.
    #[error("[Timeout] {0}")]
    Timeout(String),

    /// An HTTP request returned a non-success status.
    /// Carries the status code, reason phrase, and response body so the
    /// caller can construct an `ApiResponse::error` without re-fetching.
    #[error("[HTTP {status}] {reason}")]
    HttpStatus {
        /// The numeric HTTP status code (e.g. `400`, `500`).
        status: u16,
        /// The canonical reason phrase (e.g. `"Bad Request"`).
        reason: String,
        /// The raw response body as a string.
        body: String,
    },

    /// JSON encoding or decoding failed.
    /// The inner `String` describes the parse error.
    #[error("[Json] {0}")]
    Json(String),

    /// A catch-all error variant for cases that don't fit the other kinds.
    /// The inner `String` is a free-form message.
    #[error("{0}")]
    Generic(String),
}

/// JSON-line envelope printed to stdout on every CLI invocation.
///
/// This is the **public contract** of the binary. Every command, success or
/// failure, prints exactly one of these as a single line of JSON. Downstream
/// CI scripts parse this from stdout.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ApiResponse {
    /// The HTTP-style status code. `200` indicates success.
    #[serde(rename = "status")]
    pub status: u16,
    /// A short, canonical reason phrase (`"OK"`, `"Bad Request"`, etc.).
    #[serde(rename = "reason")]
    pub reason: String,
    /// The payload. May be a string, object, array, number, bool, or null.
    #[serde(rename = "body")]
    pub body: serde_json::Value,
}

impl ApiResponse {
    /// Build a success envelope: `status: 200, reason: "OK"`.
    pub fn ok(body: serde_json::Value) -> Self {
        Self {
            status: 200,
            reason: "OK".into(),
            body,
        }
    }

    /// Build a failure envelope with caller-supplied status and reason.
    pub fn error(status: u16, reason: &str, body: serde_json::Value) -> Self {
        Self {
            status,
            reason: reason.into(),
            body,
        }
    }

    /// Render to a single-line JSON value (no trailing newline).
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"status":1,"reason":"Bad Request","body":"json encode failed"}"#.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn api_response_round_trip_object_body() {
        let r = ApiResponse::ok(json!({"hello": "world"}));
        let line = r.to_line();
        assert!(line.contains("\"status\":200"));
        assert!(line.contains("\"reason\":\"OK\""));
        assert!(line.contains("\"body\":{\"hello\":\"world\"}"));
        let back: ApiResponse = serde_json::from_str(&line).unwrap();
        assert_eq!(back.status, 200);
        assert_eq!(back.reason, "OK");
    }

    #[test]
    fn api_response_round_trip_string_body() {
        let r = ApiResponse::ok(json!("[Action:Success] Response successful."));
        let line = r.to_line();
        assert_eq!(
            line,
            r#"{"status":200,"reason":"OK","body":"[Action:Success] Response successful."}"#
        );
    }

    #[test]
    fn api_response_round_trip_array_body() {
        let r = ApiResponse::ok(json!([1, 2, 3]));
        assert!(r.to_line().contains("\"body\":[1,2,3]"));
    }
}
