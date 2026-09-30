//! reqwest-backed implementation of the `HttpClient` trait.
//!
//! Mirrors the PHP `CurlHelper::curlSetup` configuration:
//!
//! - 120 second request timeout (`client.timeout = 120.0`)
//! - Follow redirects (`CURLOPT_FOLLOWLOCATION = true`)
//! - Skip TLS verification (`CURLOPT_SSL_VERIFYHOST = 0`,
//!   `CURLOPT_SSL_VERIFYPEER = false`)
//! - `Authorization: Basic base64(user:password)` for credential-based auth
//! - Per-call `Content-Type` (defaults to `application/json`)

use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;

use super::{HttpClient, Response};
use crate::error::Error;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_CONTENT_TYPE: &str = "application/json";

/// reqwest-backed HTTP client.
pub struct ReqwestHttpClient {
    client: reqwest::blocking::Client,
    endpoint: String,
    authorization: Option<String>,
    content_type: String,
}

impl ReqwestHttpClient {
    /// Build a new client. Returns an error if reqwest cannot be configured.
    pub fn new() -> Result<Self, Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::limited(10))
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| Error::Generic(format!("[HttpClient] build failed: {e}")))?;
        Ok(Self {
            client,
            endpoint: String::new(),
            authorization: None,
            content_type: DEFAULT_CONTENT_TYPE.into(),
        })
    }
}

impl HttpClient for ReqwestHttpClient {
    fn set_endpoint(&mut self, endpoint: &str) {
        self.endpoint = endpoint.to_string();
    }

    fn set_authorization(&mut self, auth: &str) {
        self.authorization = Some(auth.to_string());
    }

    fn set_content_type(&mut self, content_type: &str) {
        self.content_type = content_type.to_string();
    }

    fn set_username_and_password(&mut self, user: &str, password: &str) {
        let raw = format!("{user}:{password}");
        let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
        self.authorization = Some(format!("Basic {encoded}"));
    }

    fn request(
        &mut self,
        method: &str,
        relative_url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<Response, Error> {
        if self.endpoint.is_empty() {
            return Err(Error::MissingEndpoint);
        }

        // Resolve the full URL by appending the relative URL to the
        // endpoint's path. PHP does `parse_url($endpoint, PHP_URL_PATH) . '/' . $relative`,
        // so we mirror that.
        let endpoint_path = url_path(&self.endpoint);
        let full = format!(
            "{}/{}",
            endpoint_path.trim_end_matches('/'),
            relative_url.trim_start_matches('/')
        );

        let method = Method::from_bytes(method.to_ascii_uppercase().as_bytes())
            .map_err(|e| Error::Generic(format!("[HttpClient] bad method: {e}")))?;

        let mut header_map = HeaderMap::new();
        header_map.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&self.content_type)
                .map_err(|e| Error::Generic(format!("[HttpClient] bad content-type: {e}")))?,
        );
        if let Some(auth) = &self.authorization {
            if let Ok(value) = HeaderValue::from_str(auth) {
                header_map.insert(AUTHORIZATION, value);
            }
        }
        if let Some(extra) = headers {
            for (k, v) in extra {
                if let (Ok(name), Ok(value)) = (
                    HeaderName::from_bytes(k.as_bytes()),
                    HeaderValue::from_str(&v),
                ) {
                    header_map.insert(name, value);
                }
            }
        }

        let url = format!("{}{}", base_url(&self.endpoint), full);
        eprintln!("[reqwest] {method} {url}");
        let mut req = self.client.request(method, &url).headers(header_map);

        req = match body {
            Some(b) if self.content_type == "application/x-www-form-urlencoded" => req.body(b),
            Some(b) => req.body(b),
            None => req,
        };

        let response = req
            .send()
            .map_err(|e| Error::Generic(format!("[HttpClient] send failed: {e}")))?;

        let status = response.status().as_u16();
        let reason = response
            .status()
            .canonical_reason()
            .unwrap_or("")
            .to_string();
        let body_text = response
            .text()
            .map_err(|e| Error::Generic(format!("[HttpClient] read body failed: {e}")))?;

        // PHP raises on 4xx/5xx. We surface the same: keep the status but
        // let the caller branch. The PHP behaviour wraps the raw body in
        // an exception with the body as the message; we mirror by returning
        // the response so callers can choose how to react. Tests confirm.
        if (400..=599).contains(&status) {
            return Err(Error::HttpStatus {
                status,
                reason,
                body: body_text,
            });
        }

        Ok(Response {
            status,
            reason,
            body: body_text,
        })
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new().expect("ReqwestHttpClient::new should succeed with default settings")
    }
}

/// Extract the path component of an endpoint URL.
fn url_path(endpoint: &str) -> String {
    match url::Url::parse(endpoint) {
        Ok(u) => u.path().to_string(),
        Err(_) => endpoint.to_string(),
    }
}

/// Extract scheme + host (no path) from an endpoint URL. Falls back to
/// `endpoint` as-is for relative paths.
fn base_url(endpoint: &str) -> String {
    match url::Url::parse(endpoint) {
        Ok(u) => {
            let host = u.host_str().unwrap_or("");
            let scheme = u.scheme();
            let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!("{scheme}://{host}{port}")
        }
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_path_extracts_path() {
        assert_eq!(url_path("https://example.com/naut"), "/naut");
        assert_eq!(url_path("https://example.com/naut/"), "/naut/");
    }

    #[test]
    fn base_url_extracts_origin() {
        assert_eq!(base_url("https://example.com/naut"), "https://example.com");
        assert_eq!(
            base_url("https://api.bitbucket.org/2.0/"),
            "https://api.bitbucket.org"
        );
    }
}
