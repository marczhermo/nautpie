//! HTTP client abstraction. The trait allows tests to substitute a mock
//! implementation that mirrors the PHP `MockHandler` injection pattern.
//!
//! Production code uses [`ReqwestHttpClient`]; tests use
//! `wiremock::MockServer` directly.

use std::collections::HashMap;

use crate::error::Error;

pub mod reqwest_client;

pub use reqwest_client::ReqwestHttpClient;

#[derive(Debug)]
/// The result of a single HTTP request.
///
/// Returned by [`HttpClient::request`]. The body is kept as a `String`
/// rather than a parsed `serde_json::Value` so the client stays focused on
/// HTTP and lets callers decide how to interpret the payload.
pub struct Response {
    /// Numeric HTTP status code (200, 404, 500, ...).
    pub status: u16,
    /// Canonical reason phrase (`"OK"`, `"Not Found"`, ...).
    pub reason: String,
    /// Raw response body as a UTF-8 string.
    pub body: String,
}

/// HTTP client abstraction.
///
/// Mirrors the PHP `CurlFetch` helper. Production code uses
/// [`ReqwestHttpClient`]; tests can substitute any other implementation.
#[allow(dead_code)]
pub trait HttpClient {
    /// Store the base URL that subsequent [`HttpClient::request`] calls
    /// will resolve relative paths against.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The base URL (e.g. `https://api.example.com/naut`).
    fn set_endpoint(&mut self, endpoint: &str);

    /// Set a raw `Authorization` header value.
    ///
    /// # Arguments
    ///
    /// * `auth` - The full header value (e.g. `"Bearer xyz"`).
    fn set_authorization(&mut self, auth: &str);

    /// Set the `Content-Type` header used by future requests.
    ///
    /// # Arguments
    ///
    /// * `content_type` - The MIME type (e.g. `"application/json"`).
    fn set_content_type(&mut self, content_type: &str);

    /// Set HTTP basic-auth credentials. The client builds the
    /// `Authorization: Basic base64(user:password)` header internally.
    ///
    /// # Arguments
    ///
    /// * `user` - The username.
    /// * `password` - The password.
    fn set_username_and_password(&mut self, user: &str, password: &str);

    /// Send an HTTP request.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method in upper case: `"GET"`, `"POST"`, etc.
    /// * `relative_url` - Path to resolve against the configured endpoint.
    /// * `headers` - Optional extra headers to add to the request.
    /// * `body` - Optional request body as a string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingEndpoint`] if [`HttpClient::set_endpoint`]
    /// was never called. Returns [`Error::Generic`] for transport failures
    /// (DNS, TLS, connection refused, etc.). Returns [`Error::HttpStatus`]
    /// for any 4xx/5xx response.
    fn request(
        &mut self,
        method: &str,
        relative_url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<Response, Error>;
}
