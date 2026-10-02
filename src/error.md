# `src/error.rs` — Errors and the JSON Response Envelope

This file defines two things that are absolutely central to the program:

1. **`Error`** — every error the program can produce, as a Rust enum.
2. **`ApiResponse`** — the JSON envelope printed on stdout for every command, success or failure.

Together, they form the **public contract** between `nautpie` and whatever script is calling it. If you're going to understand one file in this codebase, make it this one.

---

## The Big Picture

Every command invocation produces exactly one JSON line on stdout with this shape:

```json
{"status": 200, "reason": "OK", "body": <whatever>}
```

- **`status`** — HTTP-like status code. `200` for success, anything else for failure.
- **`reason`** — short human-readable label, like `"OK"` or `"Bad Request"`.
- **`body`** — the actual payload. Could be a string, an object, an array, `null` — anything.

When something goes wrong, the program returns `Err(Error::SomeVariant)` from the action function. The dispatcher in `main.rs` catches it and converts it into an `ApiResponse::error(...)` that still has the same three-field shape, so CI scripts can parse stdout uniformly.

---

## Walkthrough

### The `Error` enum

```rust
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
```

This is a **sum type** (a Rust enum with data attached). Each variant represents a different kind of failure:

- **`MissingEnv(String)`** — a required environment variable wasn't set. The `String` is the variable's name. This is a **tuple variant**: it carries an unnamed field.
- **`MissingOption(String)`** — a required CLI option wasn't supplied. Same shape.
- **`MissingAction`** — a unit variant: no data at all, just the tag.
- **`MissingEndpoint`** — the API endpoint env var wasn't set.
- **`Timeout(String)`** — a polling operation exceeded its deadline. The `String` is the descriptive message.
- **`HttpStatus { status, reason, body }`** — a **struct variant**: it has named fields, like a tiny struct. The HTTP request returned a non-success status.
- **`Json(String)`** — JSON encoding or decoding failed.
- **`Generic(String)`** — a catch-all for anything else.

**Teaching points:**

- **`#[derive(Error)]`** — comes from the `thiserror` crate. It generates an `impl std::error::Error for Error` (and an `impl Display for Error`) automatically, so `Error` integrates with Rust's standard error-handling ecosystem.
- **`#[error("...")]`** — the format string for the error's `Display` implementation. `{0}` is the first field, `{name}` is the named field. When you `println!("{}", err)`, you get that formatted string.
- **`PartialEq, Eq`** — derives for equality. Two `Error` values can be compared with `==`. The tests at the bottom rely on this.
- **Tuple variants vs. struct variants** — Both hold data. Tuple variants are concise (`MissingEnv(String)`); struct variants are self-documenting (`HttpStatus { status, reason, body }`). Use struct variants when there are 2+ fields and the names matter.

### Why `thiserror`?

In plain Rust, you'd write:

```rust
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingEnv(name) => write!(f, "[Required:ENV] {name} is missing."),
            // ... 7 more arms ...
        }
    }
}

impl std::error::Error for Error {}
```

The `#[derive(Error)]` macro generates all of that for you from the `#[error("...")]` attributes. Less boilerplate, less chance of forgetting an arm.

### The `ApiResponse` struct

```rust
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ApiResponse {
    #[serde(rename = "status")]
    pub status: u16,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "body")]
    pub body: serde_json::Value,
}
```

This is the **envelope** that goes on stdout. Three fields:

- **`status: u16`** — an unsigned 16-bit integer (0–65535). Real HTTP status codes fit easily in this range.
- **`reason: String`** — a short label like `"OK"`, `"Bad Request"`, `"Internal Server Error"`.
- **`body: serde_json::Value`** — the actual payload. `serde_json::Value` is an **enum** that can be any JSON value: `Null`, `Bool`, `Number`, `String`, `Array`, or `Object`. This is how we get a "body of any shape" without writing a different struct for every action.

**Teaching points:**

- **`#[derive(Serialize, Deserialize)]`** — auto-generates code to convert this struct to and from JSON. Without these, you couldn't print it.
- **`#[serde(rename = "status")]`** — already redundant here because the field is already named `status`, but it's kept explicit so the JSON key matches even if the field is renamed in code.
- **`Clone`** — lets you duplicate an `ApiResponse` cheaply. Useful for tests.

### The constructor methods

```rust
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

    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"status":1,"reason":"Bad Request","body":"json encode failed"}"#.to_string()
        })
    }
}
```

- **`ok(body)`** — convenience for the success case. Always produces `status=200, reason="OK"`.
- **`error(status, reason, body)`** — convenience for failures. The caller picks the status code and reason.
- **`to_line(&self) -> String`** — serializes to a single line of JSON (no newline at the end). The `unwrap_or_else` clause returns a fallback if serialization itself fails (essentially "couldn't even encode the error").
- **`"OK".into()`** — the `From<&str> for String` conversion. Shorter than `"OK".to_string()`.

### Tests

The `#[cfg(test)]` block tests that `ApiResponse` round-trips correctly through JSON for all three body shapes (object, string, array). This is important because if the envelope shape ever changes, downstream CI scripts will silently break.

---

## Why Is It Written This Way?

- **Single JSON line, always.** Whether the command succeeds or fails, the caller gets exactly one parseable JSON line on stdout. This is the **PHP parity contract** — the original PHP version did the same thing.
- **`Error` as an enum, not a string.** Rust strongly prefers typed errors. Every function returning `Result<T, Error>` lets the caller pattern-match on the specific failure: `match err { Error::MissingEnv(name) => ..., Error::Timeout(_) => ... }`. Compare with a stringly-typed error where you'd have to parse the message text.
- **`#[error("...")]` for messages.** The PHP original prefixed each error kind (`[Required:ENV]`, `[Timeout]`, etc.). The Rust port keeps those prefixes so the error text is byte-identical, making migration painless.
- **`body: serde_json::Value`.** Each action returns completely different data. Rather than write a generic `ApiResponse<T>` (which would force every caller to deal with generics), the body is `Value` — Rust's "any JSON" type.
- **`thiserror` over manual `impl`.** Less boilerplate, less chance of typos in the `Display` impls, automatic `std::error::Error` integration.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `enum` with data | `Error` | A type with a fixed set of variants, each possibly carrying data |
| Tuple variant | `MissingEnv(String)` | Variant carrying unnamed fields |
| Struct variant | `HttpStatus { status, reason, body }` | Variant carrying named fields |
| Unit variant | `MissingAction` | Variant carrying no data |
| `#[derive(...)]` | everywhere | Auto-generates trait implementations |
| `thiserror::Error` | `Error` | Generates `Display` + `std::error::Error` impls |
| `serde::Serialize` | `ApiResponse` | Makes a type serializable to JSON |
| `serde::Deserialize` | `ApiResponse` | Makes a type deserializable from JSON |
| `serde_json::Value` | `body` field | A type that can hold any JSON value |
| `Into` / `From` | `"OK".into()` | Type conversion via trait |
| `unwrap_or_else(\|_\| ...)` | `to_line` | "If this fails, run this fallback" |
| `Result<T, E>` | callers | The standard success-or-failure return type |
| `#[cfg(test)]` | test module | "Only compile in test mode" |
