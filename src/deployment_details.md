# `src/deployment_details.rs` — The Fluent Deployment Payload Builder

This file defines `DeploymentDetails`, a **builder-pattern struct** for constructing the JSON payload that gets POSTed to DeployNaut when you create a new deployment. It's the spiritual equivalent of the PHP `DeploymentDetails` class.

---

## The Big Picture

When DeployNaut's API receives a `POST /project/{stack}/environment/{env}/deploys`, it expects a JSON body like:

```json
{
  "ref": "abc123def456...",
  "ref_type": "sha",
  "title": "[CI] Deployment",
  "summary": "Branch:main",
  "bypass_and_start": true,
  "locked": false
}
```

The `DeploymentDetails` struct lets you build that body step by step:

```rust
let details = DeploymentDetails::new()
    .ref_("abc123def456...")
    .ref_type("sha")
    .title("[CI] Deployment")
    .summary("Branch:main")
    .bypass_and_start(true);

let json = details.values();  // → serde_json::Value
```

Each method returns `Self` (i.e., a modified copy of the struct), so the calls can be chained. This is the **builder pattern**, and it's one of the most popular idioms in Rust.

---

## Walkthrough

### Imports

```rust
use chrono::DateTime;
use serde_json::{Map, Value};
```

- `chrono::DateTime` — a date-and-time type from the `chrono` crate. Used to parse RFC3339 timestamps.
- `serde_json::{Map, Value}` — `Map` is the type of an object's key-value pairs (a wrapper around `BTreeMap` or `HashMap` depending on features); `Value` is the "any JSON" enum.

### Constant

```rust
const DEFAULT_TITLE: &str = "[CI] Deployment";
```

A `const` is a value the compiler **inlines at every use site**. Unlike `let`, you can't have a mutable `const`, and the value must be known at compile time. Here it's just a default string.

The leading `[CI]` is a tag the DeployNaut UI shows in its dashboard. Production deployments get `[CD]` (continuous deployment) instead, but this struct defaults to `[CI]` to match the PHP original.

### The struct

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentDetails {
    pub r#ref: Option<String>,
    pub ref_type: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub bypass: bool,
    pub bypass_and_start: bool,
    pub schedule_start_unix: Option<i64>,
    pub schedule_end_unix: Option<i64>,
    pub locked: bool,
}
```

This is a plain data struct. Each field is `pub` so the constructor and methods (which live in the same module) can read and write them freely.

- **`r#ref`** — the raw-identifier escape again (the same trick we saw in `cli.rs`). `ref` is a reserved keyword in Rust, so we prefix it with `r#`. The actual JSON key will be `"ref"`.
- **`Option<String>`** for optional fields, plain `String` for required defaults, plain `bool` for booleans.
- **`i64`** for Unix timestamps (signed 64-bit integer). Unix epoch fits in `i64` until the year 292 billion, so we're safe.
- **`#[derive(PartialEq)]`** — generates `==` for the struct. Tests use this.
- **`#[derive(Clone)]`** — generates `.clone()`, useful for passing a copy of the payload around.

### `Default` impl

```rust
impl Default for DeploymentDetails {
    fn default() -> Self {
        Self::new()
    }
}
```

This says: "the default `DeploymentDetails` is the same as a freshly-constructed one." It just delegates to `new()`. The derive macro couldn't auto-generate this for us because the defaults involve `Some(String::new())` and `Some("[CI] Deployment".into())` — values the macro can't infer.

### `new()`

```rust
pub fn new() -> Self {
    Self {
        r#ref: Some(String::new()),
        ref_type: "sha".into(),
        title: Some(DEFAULT_TITLE.into()),
        summary: Some(String::new()),
        bypass: false,
        bypass_and_start: false,
        schedule_start_unix: None,
        schedule_end_unix: None,
        locked: false,
    }
}
```

**Teaching points:**

- **`Some(String::new())`** — wraps an empty string in `Some`. The struct starts with empty (but `Some`) optional fields so the builder can mutate them.
- **`"sha".into()`** — converts the `&str` literal to a `String`. The `into()` method works because `&str` implements `Into<String>`.
- **`Some(DEFAULT_TITLE.into())`** — the constant is `&str`, so we need `.into()` to make it `String` before wrapping in `Some`.
- **`.into()` vs `String::from()` vs `.to_string()`** — all three convert a `&str` to a `String`. They're interchangeable here; `into()` is just shorter.

### Builder methods

Each builder method has the same shape:

```rust
pub fn ref_type(mut self, value: impl Into<String>) -> Self {
    self.ref_type = value.into();
    self
}
```

**Teaching points:**

- **`mut self`** — takes ownership of `self` (not a borrow). This is what enables the chaining pattern.
- **`impl Into<String>`** — same flexibility we saw in `Options::set()`. Callers can pass `&str`, `String`, or anything else convertible.
- **`value.into()`** — performs the conversion. Note we don't call `.to_string()` here because the compiler can't infer the target type from context; `into()` works because the destination type (`String`) is known.
- **Returns `self`** — returns the modified struct by value. The caller receives ownership and can call the next builder method.

### `schedule_to_start` and `schedule_end`

```rust
pub fn schedule_to_start(mut self, time_str: impl AsRef<str>) -> Self {
    let raw = time_str.as_ref();
    self.schedule_start_unix = parse_unix(raw);
    self
}
```

- **`impl AsRef<str>`** — similar to `Into<String>` but a different trait. `AsRef` is for **borrowing** as a different type. Both `&str` and `String` implement `AsRef<str>` (the former is already a `&str`; the latter can be borrowed as one). This is more flexible than taking `&str` directly.
- **`time_str.as_ref()`** — converts whatever the caller passed into `&str`.
- **`parse_unix(raw)`** — calls a private helper at the bottom of the file that parses the time string.

### `redeploy`

```rust
pub fn redeploy(mut self, yes: bool) -> Self {
    if yes {
        self.r#ref = Some(String::new());
        self.ref_type = "redeploy".into();
    }
    self
}
```

The `if yes` block only runs when the flag is truthy. If you call `.redeploy(false)`, nothing changes. If you call `.redeploy(true)`, the ref is wiped and the ref_type becomes `"redeploy"`.

This is the **PHP `DeploymentDetails::redeploy($yes)`** semantics: when redeploying, you don't want to specify a commit SHA (you're re-running the previous one), so the ref is left empty and the API knows to use the last deployment.

### `values()` — the serializer

```rust
pub fn values(&self) -> Value {
    let mut map = Map::new();
    let mut insert = |key: &str, val: Value| {
        if !matches!(val, Value::Null) {
            map.insert(key.into(), val);
        }
    };

    match &self.r#ref {
        Some(s) if !s.is_empty() => insert("ref", Value::String(s.clone())),
        _ => {}
    }

    insert("ref_type", Value::String(self.ref_type.clone()));
    insert("title", /* similar logic */);
    // ... and so on ...
    Value::Object(map)
}
```

This is the **rendering** method. It walks the struct, builds a `serde_json::Map<String, Value>` (the equivalent of a JSON object), and wraps it in `Value::Object(...)`.

**Teaching points:**

- **`Map::new()`** — creates an empty JSON object.
- **The `insert` closure** is a private helper that only inserts if the value is not `Value::Null`. This is how empty/null fields are stripped — it mirrors PHP's `array_filter($details, 'isNotNull')`.
- **The `match` for `r#ref`** has a **pattern guard** (`if !s.is_empty()`). If the ref is set to a non-empty string, insert it; otherwise, skip it entirely (don't even insert a null).
- **`s.clone()`** — clones the `&String` into a new owned `String` so we can move it into the `Value::String` constructor.
- **`Value::Bool(...)`**, **`Value::String(...)`**, **`Value::Number(...).into()`** — these are variants of the `serde_json::Value` enum. Each constructor takes the right type and wraps it.

### `parse_unix()`

```rust
fn parse_unix(raw: &str) -> Option<i64> {
    if let Ok(n) = raw.parse::<i64>() {
        return Some(n);
    }
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}
```

A private helper function (note: no `pub`, so it's only callable from within this module).

**Teaching points:**

- **`raw.parse::<i64>()`** — turbofish syntax to disambiguate the generic type. We're telling the compiler "parse this `&str` into an `i64`." Returns `Result<i64, ParseIntError>`.
- **`if let Ok(n) = ...`** — pattern-match on the `Result`. If it's `Ok`, bind the inner value to `n` and run the body; if it's `Err`, do nothing (and fall through to the next line).
- **`DateTime::parse_from_rfc3339(raw)`** — from `chrono`. Parses an RFC3339 timestamp string like `"2026-12-25T10:00:00Z"`. Returns `Result<DateTime<FixedOffset>, ParseError>`.
- **`.ok()`** — converts `Result<T, E>` to `Option<T>`. `Err` becomes `None`.
- **`.map(|dt| dt.timestamp())`** — if the `Option` is `Some(dt)`, apply the closure to get the Unix timestamp; if it's `None`, leave it as `None`. The chained `.ok().map(...)` is a very common Rust idiom for "try this, then transform if successful."

---

## Why Is It Written This Way?

- **Builder pattern.** Lets you construct the payload incrementally, only setting the fields you care about. Compare with a constructor that takes 9 positional arguments.
- **PHP parity, with one fix.** The PHP `DeploymentDetails::summery()` (note the typo) meant the `summary` field was never persisted. The Rust port corrects the typo to `summary` so the field actually makes it into the payload.
- **`values()` strips empty fields.** PHP used `array_filter($details, 'isNotNull')` to drop null entries. The Rust version replicates this exactly so the wire format is identical.
- **Builder methods return `Self`, not `&mut Self`.** This is the **consuming builder** pattern: each method takes ownership and returns the modified value. It prevents two builders from sharing the same struct, which would lead to confusing bugs. The trade-off is that each call allocates, but for a small struct like this, the cost is negligible.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `const NAME: &str = "..."` | top | Compile-time constant |
| `pub struct` | `DeploymentDetails` | Public data struct |
| `Option<String>` / `Option<i64>` | optional fields | "Some value, or nothing" |
| `bool` | flag fields | `true` / `false` |
| `r#name` | `r#ref` | Raw identifier — escape a reserved word |
| `impl Default for Struct` | manual impl | "Default value when `::default()` is called" |
| `mut self` | builder methods | Take ownership of `self` to modify it |
| `impl Into<String>` | builder params | Accepts anything convertible to `String` |
| `impl AsRef<str>` | schedule params | Accepts anything that can be borrowed as `&str` |
| `self` returned | every builder | Returns the modified struct by value |
| Closure `\|key, val\|` | `insert` helper | Inline function with two arguments |
| Pattern guard `if !s.is_empty()` | `r#ref` match | Refine a pattern with an extra condition |
| `serde_json::Value` | `values()` return | "Any JSON" type |
| `serde_json::Map` | inside `values()` | JSON object type |
| `Value::Object(map)` | wrap the map | Construct a JSON object value |
| `chrono::DateTime::parse_from_rfc3339` | `parse_unix` | Parse an RFC3339 timestamp |
| `.parse::<i64>()` | `parse_unix` | Parse a string into an integer |
| Turbofish `::<T>` | `raw.parse::<i64>()` | Disambiguate a generic type |
| `if let Ok(n) = ...` | `parse_unix` | Match only the `Ok` arm |
| `.ok()` | `parse_unix` | `Result<T, E>` → `Option<T>` |
| `.map(\|dt\| ...)` | `parse_unix` | Transform inside an `Option` |
| `#[derive(...)]` | struct | Auto-generate trait impls |
