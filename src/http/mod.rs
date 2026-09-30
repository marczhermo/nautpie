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
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub body: String,
}

#[allow(dead_code)]
pub trait HttpClient {
    fn set_endpoint(&mut self, endpoint: &str);
    fn set_authorization(&mut self, auth: &str);
    fn set_content_type(&mut self, content_type: &str);
    fn set_username_and_password(&mut self, user: &str, password: &str);

    fn request(
        &mut self,
        method: &str,
        relative_url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<Response, Error>;
}
