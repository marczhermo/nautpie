# `tests/http_test.rs` — Wiremock-Driven HTTP Client Tests

This file contains integration tests for the production `ReqwestHttpClient`. It uses **`wiremock`** to spin up a real HTTP server in-process, then runs the actual `reqwest` client against it. This is the Rust equivalent of the PHP `MockHandler` test pattern.

---

## The Big Picture

The tests verify that `ReqwestHttpClient` correctly:

1. Sends GETs and parses status / reason / body.
2. Sends POSTs with JSON bodies.
3. Sends POSTs with form-urlencoded bodies.
4. Maps 4xx HTTP responses to `Error::HttpStatus`.
5. Base64-encodes basic-auth credentials.
6. Returns `MissingEndpoint` if `set_endpoint` was never called.
7. Merges extra headers into the request.

Each test sets up its own mock server, mounts its own expectations, and runs the client against it.

---

## Walkthrough

### Imports

```rust
use std::collections::HashMap;

use base64::Engine;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use nautpie::error::Error;
use nautpie::http::{HttpClient, ReqwestHttpClient};
```

- **`wiremock::*`** — the mock HTTP server. Key types:
  - **`MockServer`** — the server itself. Started with `MockServer::start().await`.
  - **`Mock::given(...).respond_with(...)`** — registers an expected request and the response to return.
  - **`MockServer::uri()`** — the base URL of the running mock server.
- **`wiremock::matchers::*`** — composable matchers for request properties:
  - **`method("GET")`** — match by HTTP method.
  - **`path("/foo")`** — match by URL path.
  - **`header("name", "value")`** — match by header.
  - **`body_string("...")`** — match by exact body string.
- **`base64::Engine`** — used to verify the basic-auth encoding.

### Helpers

#### `build_client()`

```rust
fn build_client(server: &MockServer) -> ReqwestHttpClient {
    let mut c = ReqwestHttpClient::new().expect("client builds");
    c.set_endpoint(&server.uri());
    c
}
```

A trivial constructor: build the client and point it at the mock server's URI.

**Teaching points:**

- **`&MockServer`** — borrowed reference. We only need to call `.uri()` on it.
- **`.expect("client builds")`** — panic if `ReqwestHttpClient::new()` returns `Err`.

#### `start_server()`

```rust
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
```

This is one of the **trickier** pieces of the test infrastructure.

**Teaching points:**

- **The `F` generic with HRTB** — `for<'a> FnOnce(&'a MockServer) -> Pin<Box<dyn Future<Output = ()> + 'a>>` is a **higher-ranked trait bound**. It says: "F is a function that, for ANY lifetime `'a`, can produce a future borrowing the `MockServer` for that lifetime." This is necessary because the `wiremock::Mock::mount(...)` function is async and borrows the server.
- **`std::pin::Pin<Box<dyn Future>>`** — a pinned, heap-allocated, type-erased future. We need `Pin` because the future might contain self-referential state. We need `Box<dyn Future>` because the concrete future type is unknown to us.
- **`tokio::runtime::Builder::new_current_thread()`** — builds a single-threaded tokio runtime. We need this because `wiremock` is async, but our `reqwest::blocking` client is sync. The two need to coexist, so we start a small tokio runtime just for the wiremock setup.
- **`rt.block_on(async { MockServer::start().await })`** — runs an async block on the runtime. `MockServer::start()` returns a future; `block_on` waits for it to complete.
- **`rt.block_on(async { setup(&server).await })`** — runs the user's setup closure (which mounts the mocks).
- **`drop(rt)`** — explicit drop of the runtime. We want the runtime to be gone before we make HTTP calls with the sync `reqwest::blocking` client.
- **`server`** — returning the `MockServer` is fine even though the runtime is dropped. The server runs in its own thread.

### Tests

#### GET round-trip

```rust
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
```

**Teaching points:**

- **`serde_json::json!({...}).to_string()`** — build a JSON value with the `json!` macro, then serialize to a string.
- **`Mock::given(method("GET")).and(path("...")).respond_with(...)`** — fluent chain that mounts an expectation. `and(...)` composes matchers with logical AND.
- **`ResponseTemplate::new(200).set_body_string(body)`** — the mock response: status 200 with the given body.
- **`.mount(s).await`** — register the mock against the server. The `.await` is required because `mount` is async.
- **`Box::pin(async move { ... })`** — wrap the async block in a pinned boxed future. The `move` keyword moves captured variables (`body`, `s`) into the future.
- **`let body = body.clone();`** — clone inside the outer closure so the inner `move` closure can take ownership.
- **`client.request("GET", "relative/url", None, None).expect("get succeeds")`** — make the actual HTTP call. `None, None` means no extra headers, no body.
- **`serde_json::from_str(&resp.body).unwrap()`** — parse the response body as JSON. `.unwrap()` panics if parsing fails (acceptable in a test).

#### POST JSON

```rust
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
```

This test mounts a mock that **only matches** if:

- The method is POST.
- The path is `/project/example/environment/uat/deploys`.
- The `Content-Type` header is `application/json`.
- The body is **exactly** `{"ref_type":"sha","ref":"abc"}`.

If the client sends anything different, the test fails with a "no matching mock" error. This is what makes wiremock powerful: it asserts on the **shape of the request**, not just the response.

**Teaching points:**

- **`body_string("...")`** — exact-match on the request body. There's also `body_partial_json(...)` for partial matching.
- **`Some(string.into())`** — the `Option<String>` parameter type requires `.into()` to convert `&str` to `String`.

#### POST form-urlencoded

```rust
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
```

Same pattern, but verifying that an `application/x-www-form-urlencoded` body is sent correctly. The client is configured with `set_content_type("application/x-www-form-urlencoded")` before the request.

#### 4xx → `Error::HttpStatus`

```rust
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
        Error::HttpStatus { status, reason, body } => {
            assert_eq!(status, 400);
            assert_eq!(reason, "Bad Request");
            assert_eq!(body, r#"{"error":"bad"}"#);
        }
        other => panic!("expected HttpStatus, got {other:?}"),
    }
}
```

**Teaching points:**

- **`.expect_err(...)`** — assert that a `Result` is `Err` and return the error. Mirror of `.expect()` which asserts `Ok`.
- **Pattern matching on `Error::HttpStatus { status, reason, body }`** — destructures the struct variant. Each field is bound by name.
- **`panic!("expected HttpStatus, got {other:?}")`** — `{:?}` is the `Debug` formatter. Prints the value for diagnostic purposes.

#### Basic auth header

```rust
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
```

This test computes the expected `Authorization` header value (base64 of `marco:password`), then mounts a mock that matches only if the header matches. Then it makes a request with `set_username_and_password("marco", "password")` and verifies the request succeeded.

**Teaching points:**

- **`base64::engine::general_purpose::STANDARD.encode(...)`** — the standard base64 alphabet (not URL-safe).
- **`move |s| { ... }`** — outer closure captures `expected` by move.
- **`let expected = expected.clone();`** — clone inside the outer closure because the inner `move` closure will take ownership.
- **`header("authorization", expected)`** — match a specific header value. Lowercase header name (`authorization`) because HTTP headers are case-insensitive.

#### Missing endpoint

```rust
#[test]
fn missing_endpoint_returns_error() {
    let mut client = ReqwestHttpClient::new().unwrap();
    let err = client
        .request("GET", "meta", None, None)
        .expect_err("no endpoint should error");
    assert!(matches!(err, Error::MissingEndpoint));
}
```

The simplest test in the file: build a client, never call `set_endpoint`, make a request, expect `MissingEndpoint`.

**Teaching points:**

- **`assert!(matches!(err, Error::MissingEndpoint))`** — `matches!` macro returns `true` if the expression matches the pattern. `assert!` then checks that the result is `true`.

#### Extra headers

```rust
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
```

Verifies that the `headers` parameter on `request(...)` is actually added to the outgoing request.

**Teaching points:**

- **`HashMap::new()`** — type-inferred as `HashMap<String, String>` from the `insert` calls.
- **`"x-custom".into()`** — `&str` → `String` conversion via the `Into` trait.

---

## Why Is It Written This Way?

- **Real HTTP server, real reqwest.** Unlike a pure mock, `wiremock` runs an actual HTTP server and `reqwest::blocking` makes actual TCP connections. This catches issues that pure mocks would miss (e.g., header case sensitivity, TLS edge cases).
- **Request-shape assertions.** The `Mock::given(...)` chain asserts that the **outgoing request** matches specific criteria. This is critical: it ensures the client is sending what we expect, not just that it handles responses correctly.
- **`expect_err` and `matches!`.** Tests assert on the **specific error variant**, not just "it failed." This catches regressions where the error type changes.
- **Helper for the awkward async boundary.** The `start_server` helper isolates the complexity of running an async setup on a sync test runner. The actual tests are short and focused.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `tests/` directory | whole file | Integration tests live here |
| `#[test]` | every test | Marks a function as a test |
| `for<'a> FnOnce(&'a ...) -> ...` | `start_server` | Higher-ranked trait bound |
| `Pin<Box<dyn Future + 'a>>` | `start_server` | Pinned, heap-allocated, type-erased future |
| `tokio::runtime::Builder` | `start_server` | Build a tokio runtime |
| `rt.block_on(future)` | `start_server` | Run a future to completion |
| `wiremock::MockServer::start()` | `start_server` | Start an in-process mock server |
| `Mock::given(method(...))` | every test | Start a request matcher chain |
| `.and(matcher)` | every test | Compose matchers with logical AND |
| `.respond_with(template)` | every test | Specify the mock response |
| `.mount(server)` | every test | Register the mock |
| `serde_json::json!(...)` | GET test | Build a JSON value |
| `.to_string()` | GET test | Serialize JSON to a string |
| `.expect_err(...)` | several tests | Assert `Result` is `Err` and return it |
| `matches!(expr, pattern)` | missing-endpoint test | "Does this expression match the pattern?" |
| `assert!(expr)` | missing-endpoint test | Assert that an expression is `true` |
| Pattern destructuring | 4xx test | `Error::HttpStatus { status, reason, body }` |
| `panic!("...{:?}", x)` | 4xx test | Panic with a debug-formatted message |
| `HashMap::new()` | extra-headers test | Create a new hash map (type-inferred) |
| `move` closure | several tests | Move captured vars into the closure |
| `.clone()` inside `move` closure | several tests | Clone outer variables for the inner closure |
| `into()` | multiple | Type conversion via `Into` |
