//! Wiremock-driven integration tests for `ReqwestHttpClient`. Mirrors the
//! role of the PHP `MockHandler` in the PHPUnit suite: spin up a real HTTP
//! server, assert that the client sends the expected request shape and
//! parses the expected response shape.
//!
//! Coverage:
//! - GET round-trip (status / reason / body)
//! - POST JSON round-trip
//! - POST application/x-www-form-urlencoded round-trip
//! - 4xx status is mapped to `Error::HttpStatus`
//! - Basic auth header is base64-encoded user:password
//!
//! Note: reqwest::blocking internally manages its own tokio runtime, so the
//! wiremock server is started inside a short-lived `block_on` block to keep
//! the two runtimes disjoint.

use std::collections::HashMap;

use base64::Engine;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use nautpie::error::Error;
use nautpie::http::{HttpClient, ReqwestHttpClient};

fn build_client(server: &MockServer) -> ReqwestHttpClient {
    let mut c = ReqwestHttpClient::new().expect("client builds");
    c.set_endpoint(&server.uri());
    c
}

fn start_server<F>(setup: F) -> MockServer
where
    F: for<'a> std::ops::FnOnce(
        &'a MockServer,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builds");
    let server = rt.block_on(async { MockServer::start().await });
    rt.block_on(async { setup(&server).await });
    drop(rt);
    server
}

#[test]
fn get_round_trip_returns_status_reason_body() {
    let body = serde_json::json!({"meta": {"whoami": "marco", "now": "2026"}}).to_string();
    let server = start_server(|s| {
        let body = body.clone();
        Box::pin(async move {
            Mock::given(method("GET"))
                .and(path("/project/example/environment/uat/deploys"))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    let resp = client
        .request("GET", "project/example/environment/uat/deploys", None, None)
        .expect("get succeeds");

    assert_eq!(resp.status, 200);
    assert_eq!(resp.reason, "OK");
    let parsed: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(parsed["meta"]["whoami"], "marco");
}

#[test]
fn post_json_sends_body() {
    let server = start_server(|s| {
        Box::pin(async move {
            Mock::given(method("POST"))
                .and(path("/project/example/environment/uat/deploys"))
                .and(header("content-type", "application/json"))
                .and(body_string(r#"{"ref_type":"sha","ref":"abc"}"#))
                .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"id":"42"}"#))
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    let resp = client
        .request(
            "POST",
            "project/example/environment/uat/deploys",
            None,
            Some(r#"{"ref_type":"sha","ref":"abc"}"#.into()),
        )
        .expect("post succeeds");

    assert_eq!(resp.status, 201);
    assert_eq!(resp.body, r#"{"id":"42"}"#);
}

#[test]
fn post_form_urlencoded_sends_body() {
    let server = start_server(|s| {
        Box::pin(async move {
            Mock::given(method("POST"))
                .and(path("/site/oauth2/access_token"))
                .and(header("content-type", "application/x-www-form-urlencoded"))
                .and(body_string("grant_type=client_credentials"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_string(r#"{"access_token":"tok"}"#),
                )
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    client.set_content_type("application/x-www-form-urlencoded");
    let resp = client
        .request(
            "POST",
            "site/oauth2/access_token",
            None,
            Some("grant_type=client_credentials".into()),
        )
        .expect("form post succeeds");

    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("access_token"));
}

#[test]
fn four_xx_returns_http_status_error() {
    let server = start_server(|s| {
        Box::pin(async move {
            Mock::given(method("GET"))
                .and(path("/meta"))
                .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"bad"}"#))
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    let err = client
        .request("GET", "meta", None, None)
        .expect_err("400 should error");

    match err {
        Error::HttpStatus {
            status,
            reason,
            body,
        } => {
            assert_eq!(status, 400);
            assert_eq!(reason, "Bad Request");
            assert_eq!(body, r#"{"error":"bad"}"#);
        }
        other => panic!("expected HttpStatus, got {other:?}"),
    }
}

#[test]
fn basic_auth_header_is_base64_encoded_user_password() {
    let raw = "marco:password";
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
    );
    let server = start_server(move |s| {
        let expected = expected.clone();
        Box::pin(async move {
            Mock::given(method("GET"))
                .and(path("/meta"))
                .and(header("authorization", expected))
                .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    client.set_username_and_password("marco", "password");
    let resp = client
        .request("GET", "meta", None, None)
        .expect("auth succeeds");
    assert_eq!(resp.status, 200);
}

#[test]
fn missing_endpoint_returns_error() {
    let mut client = ReqwestHttpClient::new().unwrap();
    let err = client
        .request("GET", "meta", None, None)
        .expect_err("no endpoint should error");
    assert!(matches!(err, Error::MissingEndpoint));
}

#[test]
fn extra_headers_are_merged() {
    let server = start_server(|s| {
        Box::pin(async move {
            Mock::given(method("GET"))
                .and(path("/meta"))
                .and(header("x-custom", "v1"))
                .respond_with(ResponseTemplate::new(200))
                .mount(s)
                .await;
        })
    });

    let mut client = build_client(&server);
    let mut extra = HashMap::new();
    extra.insert("x-custom".into(), "v1".into());
    let resp = client
        .request("GET", "meta", Some(extra), None)
        .expect("extra headers accepted");
    assert_eq!(resp.status, 200);
}
