//! Top-level error type and the JSON response envelope that the binary prints
//! on stdout. The shape mirrors the PHP version: `{"status", "reason", "body"}`,
//! where `body` may itself be a string, object, or array.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("[Required:ENV] {0} is missing.")]
    MissingEnv(String),

    #[error("[Required:Option] {0} is missing.")]
    MissingOption(String),

    #[error("[Missing] Action or End Point.")]
    MissingAction,

    #[error("[Missing] End Point is not configured.")]
    MissingEndpoint,

    #[error("[Timeout] {0}")]
    Timeout(String),

    #[error("[HTTP {status}] {reason}")]
    HttpStatus {
        status: u16,
        reason: String,
        body: String,
    },

    #[error("[Json] {0}")]
    Json(String),

    #[error("{0}")]
    Generic(String),
}

/// JSON-line envelope printed to stdout on every CLI invocation.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ApiResponse {
    #[serde(rename = "status")]
    pub status: u16,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "body")]
    pub body: serde_json::Value,
}

impl ApiResponse {
    pub fn ok(body: serde_json::Value) -> Self {
        Self {
            status: 200,
            reason: "OK".into(),
            body,
        }
    }

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
