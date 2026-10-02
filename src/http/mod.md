# `src/http/mod.rs` — The HTTP Client Abstraction

This file defines the **interface** that the action functions use to make HTTP requests. It's tiny — most of the implementation is in `reqwest_client.rs` — but it teaches a critical Rust pattern: **trait-based dependency injection**.

---

## The Big Picture

The action functions in `commands/deploy_naut.rs` and `commands/bitbucket.rs` all take a `&mut dyn HttpClient` parameter. They call methods like `set_endpoint`, `set_content_type`, and `request` on it.

This file defines:

1. **`Response`** — what an HTTP call returns.
2. **`HttpClient` trait** — the interface every HTTP client must satisfy.
3. **Re-exports** — `ReqwestHttpClient` (the production impl) is re-exported from here.

In production, the only implementor is `ReqwestHttpClient`. In tests, the same trait can be implemented by mock clients.

---

## Walkthrough

### Imports

```rust
use std::collections::HashMap;
use crate::error::Error;
```

- `HashMap<String, String>` — we'll use this for optional HTTP headers.
- `Error` — our crate's error type.

### Module and re-exports

```rust
pub mod reqwest_client;

pub use reqwest_client::ReqwestHttpClient;
```

- **`pub mod reqwest_client;`** — declares the submodule. The actual implementation lives in `reqwest_client.rs` next to this file.
- **`pub use reqwest_client::ReqwestHttpClient;`** — re-exports the production client at the `http::` level. Callers can write `use nautpie::http::ReqwestHttpClient;` instead of `use nautpie::http::reqwest_client::ReqwestHttpClient;`. It's purely a convenience.

### The `Response` struct

```rust
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub body: String,
}
```

A simple data struct with three fields:

- **`status: u16`** — the HTTP status code (200, 404, 500, etc.). `u16` is "unsigned 16-bit integer," range 0–65535.
- **`reason: String`** — the canonical reason phrase: `"OK"`, `"Not Found"`, `"Internal Server Error"`, etc.
- **`body: String`** — the response body as text. Note: it's `String`, not `serde_json::Value`. The client doesn't try to parse JSON — that's the caller's job. This keeps the client minimal and lets callers decide how to interpret the body.

`#[derive(Debug)]` lets us print the response for debugging.

### The `HttpClient` trait

```rust
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
```

This is the **interface** every HTTP client must implement. It has five methods.

**Teaching points:**

- **`pub trait HttpClient`** — defines a trait. Any struct that wants to act as an HTTP client must implement these methods.
- **`&mut self`** on every method — the client is **stateful** (it stores the endpoint, content type, auth credentials). Each method mutates that state, so we take a mutable borrow.
- **`set_endpoint(&mut self, endpoint: &str)`** — store the base URL. Subsequent calls to `request` will resolve relative URLs against this.
- **`set_authorization(&mut self, auth: &str)`** — set a raw `Authorization` header value. Useful for tokens like `Bearer xyz`.
- **`set_content_type(&mut self, content_type: &str)`** — set the `Content-Type` header. Defaults to `application/json` but can be overridden for form-encoded requests.
- **`set_username_and_password(&mut self, user: &str, password: &str)`** — convenience method: builds the basic-auth header value (`Basic base64(user:password)`) and stores it.
- **`request(&mut self, method, relative_url, headers, body) -> Result<Response, Error>`** — the main entry point. Takes an HTTP method, a path (relative to the endpoint), optional extra headers, and an optional body. Returns a `Result` because requests can fail (network errors, timeouts, etc.).
- **`#[allow(dead_code)]`** — some of these methods (like `set_authorization`) may not be called by all clients. The trait defines the full interface even if some implementors don't need every method.

### Why a trait?

Without the trait, every action function would have to know about `ReqwestHttpClient` specifically. Tests would need to either:

1. Spin up a real HTTP server (slow, brittle).
2. Refactor every action function to take an internal `Box<dyn ...>`.
3. Or just use the real client against `wiremock` (which is what we do).

With the trait, **dependency injection** is built into the type system. We can substitute any `HttpClient` impl at the call site. The integration tests use this to pass a `wiremock`-backed client instead of a real one.

### Trait objects (`dyn HttpClient`)

When you write `&mut dyn HttpClient`, you're saying: "a mutable reference to some heap-allocated value that implements `HttpClient`." Rust resolves method calls at runtime through a **vtable** (a hidden lookup table).

The alternative would be generics:

```rust
fn request<H: HttpClient>(http: &mut H, ...)
```

Generics resolve at **compile time** (monomorphization) and have no runtime cost, but they require the implementor type to be known at the call site. Trait objects resolve at **runtime** and let us pass anything implementing the trait, at the cost of one indirection per call.

This codebase uses `dyn HttpClient` because the action functions are called from a single dispatch site (`main.rs`), and we want the flexibility to swap implementations at that site without touching every action.

---

## Why Is It Written This Way?

- **PHP parity through abstraction.** The PHP original used Guzzle with a `MockHandler` for tests. The Rust port mimics that with a trait-based mock. The interface is small (5 methods) because that's all the actions need.
- **Mutable client state.** The PHP Guzzle client also had mutable config (base URL, headers, etc.). The Rust port preserves that interface, so the action functions can do `http.set_endpoint(...); http.set_content_type(...); http.request(...)` in sequence without re-passing config.
- **`String` body, not `serde_json::Value`.** The body is passed in and returned as `String`. Parsing and serializing is the caller's responsibility. This keeps the HTTP client focused on HTTP, not on JSON.
- **`#[allow(dead_code)]`** — the trait lists more methods than strictly needed today (e.g., `set_authorization`). Defining them now means callers can opt into them later without changing the trait.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub mod name;` | top | Declare a submodule |
| `pub use path::Item;` | top | Re-export an item at a shorter path |
| `pub struct Response { ... }` | struct | Define a data struct |
| `#[derive(Debug)]` | struct | Auto-implement `Debug` for printing |
| `pub trait HttpClient` | trait | Define an interface |
| `&mut self` | trait methods | Method takes a mutable borrow |
| `&str` parameter | string args | Borrowed string slice |
| `Option<HashMap<...>>` | `request` param | Optional extra headers |
| `Option<String>` | `request` param | Optional request body |
| `Result<Response, Error>` | `request` return | Success-or-failure |
| `dyn HttpClient` | callers | Trait object — runtime polymorphism |
| `#[allow(dead_code)]` | trait | Suppress unused-code warnings |
