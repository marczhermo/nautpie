# `src/http/reqwest_client.rs` — The Production HTTP Client

This file is the **production implementation** of the `HttpClient` trait. It uses the `reqwest` crate to actually make HTTP calls. While `http/mod.rs` defines the interface, this file is where the bytes hit the wire.

---

## The Big Picture

`ReqwestHttpClient` is a **stateful** wrapper around `reqwest::blocking::Client`. It holds:

- The base endpoint URL.
- The current `Authorization` header value.
- The current `Content-Type` value.

Each `request` call resolves the relative URL against the endpoint, applies the configured headers, sends the request, and returns a `Response`.

---

## Walkthrough

### Imports

```rust
use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;

use super::{HttpClient, Response};
use crate::error::Error;
```

- `Duration` — for the 120-second timeout.
- `base64::Engine` — provides the `.encode(...)` method for basic auth.
- `reqwest::header::*` — typed HTTP header constants and types. `AUTHORIZATION` and `CONTENT_TYPE` are the two header constants we'll set. `HeaderMap` is a typed collection of headers.
- `reqwest::Method` — an enum representing HTTP methods (`GET`, `POST`, etc.). More type-safe than passing strings around.
- `super::{HttpClient, Response}` — imports from the parent module (`http/mod.rs`).

### Constants

```rust
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_CONTENT_TYPE: &str = "application/json";
```

Two constants:

- **`REQUEST_TIMEOUT: Duration`** — 120 seconds. Same as the PHP `CURLOPT_TIMEOUT = 120` setting in the original.
- **`DEFAULT_CONTENT_TYPE: &str`** — JSON by default. Callers can override with `set_content_type` (e.g., for OAuth form-encoded requests).

### The struct

```rust
pub struct ReqwestHttpClient {
    client: reqwest::blocking::Client,
    endpoint: String,
    authorization: Option<String>,
    content_type: String,
}
```

Four fields:

- **`client: reqwest::blocking::Client`** — the underlying `reqwest` client. We use the **blocking** client (not async) to match the PHP version's synchronous behavior.
- **`endpoint: String`** — the base URL. Empty until `set_endpoint` is called.
- **`authorization: Option<String>`** — the `Authorization` header value, if any. `None` means "no auth header." Stored as a full string like `"Basic dXNlcjpwYXNz"` or `"Bearer xyz"`.
- **`content_type: String`** — the `Content-Type` header value. Defaults to `application/json`.

**Teaching points:**

- **`reqwest::blocking::Client`** — the synchronous flavor of `reqwest`. The async version is `reqwest::Client` (without `blocking`).
- **`Option<String>`** for `authorization` because the absence of a value is meaningful: don't send the header at all.

### `new()`

```rust
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
```

This builds the underlying `reqwest` client with specific configuration:

**Teaching points:**

- **`Client::builder()`** — the builder pattern. Each chained method configures one option.
- **`.timeout(REQUEST_TIMEOUT)`** — 120-second total timeout per request.
- **`.redirect(reqwest::redirect::Policy::limited(10))`** — follow up to 10 redirects. Matches PHP's `CURLOPT_FOLLOWLOCATION = true`.
- **`.danger_accept_invalid_certs(true)`** — accept invalid TLS certificates. **This is a security trade-off** — it matches the PHP original's `CURLOPT_SSL_VERIFYPEER = false`, but in production you'd probably want to remove this. The naming `danger_*` is a hint from the `reqwest` maintainers that these methods are risky.
- **`.danger_accept_invalid_hostnames(true)`** — same trade-off, for hostname verification.
- **`.build()`** — finalize the builder and return `Result<Client, Error>`.
- **`.map_err(|e| Error::Generic(format!("[HttpClient] build failed: {e}")))`** — convert reqwest's build error into our error type.
- **`DEFAULT_CONTENT_TYPE.into()`** — convert `&str` to `String` for the struct field.

### The `HttpClient` impl

```rust
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
    // ...
}
```

**Teaching points:**

- **`impl HttpClient for ReqwestHttpClient`** — implements the trait. The compiler verifies that every required method is present.
- **`endpoint.to_string()`** — convert `&str` to `String`. The struct needs an owned `String`.
- **`Some(auth.to_string())`** — wraps the new value in `Some(...)` since `authorization` is `Option<String>`.
- **`base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())`** — encode the `user:password` string as base64. The `Engine` trait provides the `.encode(...)` method.

### `request()` — the main entry point

```rust
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

    if (400..=599).contains(&status) {
        return Err(Error::HttpStatus {
            status,
            reason,
            body: body_text,
        });
    }

    Ok(Response { status, reason, body: body_text })
}
```

This is the meatiest method. Let's break it down section by section.

#### 1. Endpoint guard

```rust
if self.endpoint.is_empty() {
    return Err(Error::MissingEndpoint);
}
```

The first thing we do is **fail fast** if `set_endpoint` was never called. Without an endpoint, we don't know where to send requests.

#### 2. URL resolution

```rust
let endpoint_path = url_path(&self.endpoint);
let full = format!(
    "{}/{}",
    endpoint_path.trim_end_matches('/'),
    relative_url.trim_start_matches('/')
);
```

`url_path` extracts just the path component of the endpoint (e.g., `/naut`). We append the relative URL to it. This mirrors PHP's behavior: `parse_url($endpoint, PHP_URL_PATH) . '/' . $relative`.

#### 3. Method parsing

```rust
let method = Method::from_bytes(method.to_ascii_uppercase().as_bytes())
    .map_err(|e| Error::Generic(format!("[HttpClient] bad method: {e}")))?;
```

`Method::from_bytes` is a typed constructor: it parses bytes like `b"GET"` into a `Method::GET`. The conversion can fail (e.g., for invalid bytes), so it returns a `Result`. We convert that into our `Error::Generic`.

The `.to_ascii_uppercase()` allows callers to pass `"get"` or `"GET"` interchangeably.

#### 4. Header construction

```rust
let mut header_map = HeaderMap::new();
header_map.insert(
    CONTENT_TYPE,
    HeaderValue::from_str(&self.content_type)
        .map_err(|e| Error::Generic(format!("[HttpClient] bad content-type: {e}")))?,
);
```

We always insert a `Content-Type` header. The value must be valid HTTP (no newlines, no non-ASCII), so we use `HeaderValue::from_str` which returns a `Result`.

```rust
if let Some(auth) = &self.authorization {
    if let Ok(value) = HeaderValue::from_str(auth) {
        header_map.insert(AUTHORIZATION, value);
    }
}
```

The `Authorization` header is only inserted if `authorization` is `Some(...)`. We silently skip it if the value is invalid HTTP (rather than failing).

```rust
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
```

Any caller-supplied extra headers are added too. Invalid header names or values are silently skipped.

#### 5. Final URL

```rust
let url = format!("{}{}", base_url(&self.endpoint), full);
```

Combines the base URL (`https://example.com`) with the resolved path (`/naut/projects/...`).

`eprintln!("[reqwest] {method} {url}");` is a debug print to stderr. It helps trace what's being sent.

#### 6. Request body

```rust
req = match body {
    Some(b) if self.content_type == "application/x-www-form-urlencoded" => req.body(b),
    Some(b) => req.body(b),
    None => req,
};
```

Both `Some(b)` arms do the same thing (set the body). The split is for **future-proofing** — if we ever need different handling for form-encoded vs other content types, the structure is already there.

This is a common Rust pattern: use a `match` with two identical arms but a different guard, leaving the implementation flexible. You can change one arm without changing the other.

#### 7. Sending

```rust
let response = req
    .send()
    .map_err(|e| Error::Generic(format!("[HttpClient] send failed: {e}")))?;
```

Actually sends the request. The error mapping converts any reqwest error into our `Error::Generic`.

#### 8. Reading the response

```rust
let status = response.status().as_u16();
let reason = response
    .status()
    .canonical_reason()
    .unwrap_or("")
    .to_string();
let body_text = response
    .text()
    .map_err(|e| Error::Generic(format!("[HttpClient] read body failed: {e}")))?;
```

- **`response.status()`** returns a `StatusCode`.
- **`.as_u16()`** converts to a plain `u16`.
- **`.canonical_reason()`** returns the standard reason phrase (e.g., `"OK"` for 200, `"Not Found"` for 404). `unwrap_or("")` gives an empty string if the status is non-standard.
- **`.text()`** reads the body as a `String`. Could fail if the body isn't valid UTF-8, but for our use case (JSON APIs), it always is.

#### 9. Error handling for HTTP 4xx/5xx

```rust
if (400..=599).contains(&status) {
    return Err(Error::HttpStatus {
        status,
        reason,
        body: body_text,
    });
}
```

If the server returned a client or server error, we surface it as `Error::HttpStatus`. This matches PHP's behavior of throwing on non-2xx responses.

The `(400..=599).contains(&status)` syntax is an **inclusive range**: 400 ≤ status ≤ 599.

#### 10. Success return

```rust
Ok(Response { status, reason, body: body_text })
```

Wrap everything in our `Response` struct.

### `Default` impl

```rust
impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new().expect("ReqwestHttpClient::new should succeed with default settings")
    }
}
```

A default constructor that **panics** if the underlying client can't be built. Useful for tests where you don't want to handle the `Result`.

### Helper functions

#### `url_path()`

```rust
fn url_path(endpoint: &str) -> String {
    match url::Url::parse(endpoint) {
        Ok(u) => u.path().to_string(),
        Err(_) => endpoint.to_string(),
    }
}
```

Extracts the path component of a URL. If the endpoint is not a valid URL (e.g., a relative path), returns it as-is.

#### `base_url()`

```rust
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
```

Extracts `scheme://host[:port]` (e.g., `https://api.bitbucket.org`). If parsing fails, returns an empty string (which would produce invalid URLs, but tests are expected to use real URLs).

### Tests

```rust
#[test]
fn url_path_extracts_path() {
    assert_eq!(url_path("https://example.com/naut"), "/naut");
    assert_eq!(url_path("https://example.com/naut/"), "/naut/");
}

#[test]
fn base_url_extracts_origin() {
    assert_eq!(base_url("https://example.com/naut"), "https://example.com");
    assert_eq!(base_url("https://api.bitbucket.org/2.0/"), "https://api.bitbucket.org");
}
```

Tests for the URL helper functions. Note that the tests don't make actual HTTP calls — they only verify the URL parsing.

---

## Why Is It Written This Way?

- **PHP parity.** Each setting (`danger_accept_invalid_certs`, `danger_accept_invalid_hostnames`, redirects, timeout) mirrors the PHP `CURLOPT_*` constants. The behavior on the wire is identical.
- **Blocking, not async.** The PHP version was synchronous. The Rust port uses `reqwest::blocking` for the same blocking semantics. Async would force the entire dispatcher to be async, which would complicate `main.rs`.
- **Header building is explicit.** We could use `reqwest`'s builder API to set headers more declaratively, but doing it manually gives us full control over validation and skipping invalid headers.
- **The `eprintln!` debug log.** Useful during development, but should be removed (or gated) before this hits production.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `reqwest::blocking::Client` | struct field | Synchronous HTTP client |
| `Client::builder()` | `new` | Builder pattern for configuration |
| `.timeout(Duration)` | `new` | Configure request timeout |
| `.redirect(Policy::limited(n))` | `new` | Configure redirect-following |
| `Result<Self, Error>` | `new` return | Success-or-failure constructor |
| `impl HttpClient for ReqwestHttpClient` | top | Implement the trait |
| `Method::from_bytes(...)` | `request` | Parse an HTTP method |
| `HeaderMap::new()` | `request` | Create an empty header map |
| `HeaderValue::from_str(...)` | `request` | Parse a header value (returns `Result`) |
| `HeaderName::from_bytes(...)` | `request` | Parse a header name (returns `Result`) |
| `header_map.insert(name, value)` | `request` | Insert a header |
| `req.body(b)` | `request` | Set the request body (returns updated builder) |
| `req.send()` | `request` | Actually send the HTTP request |
| `.status().as_u16()` | `request` | Extract HTTP status as a `u16` |
| `.canonical_reason()` | `request` | Get the standard reason phrase |
| `.text()` | `request` | Read response body as a `String` |
| `(400..=599).contains(&status)` | `request` | Inclusive range check |
| `base64::Engine::encode` | `set_username_and_password` | Base64-encode credentials |
| `url::Url::parse(...)` | helpers | Parse a URL string |
| `Duration::from_secs(n)` | `new` | Create a duration from seconds |
| `.expect(msg)` | `default` | Panic with message on `Err` |
| `eprintln!(...)` | `request` | Print to stderr (debug) |
| `match ... { ... }` | `request` body | Match on a value (with optional guards) |
| `if let Some(v) = ...` | `request` | Pattern-match only the `Some` arm |
| `unwrap_or("")` | `request` | Default value on `None` |
| `unwrap_or_default()` | `request` | Default to the type's `Default::default()` |
| `impl Default for Struct` | bottom | Implement the `Default` trait |
