# `src/commands/sample.rs` — The Sample Smoke-Test Actions

This is the smallest `commands/` file. It implements the two **trivially-simple** actions: `sampleSuccess` and `sampleFail`. They exist to:

1. Verify that the CLI dispatch system works end-to-end.
2. Give CI scripts a way to test the binary without making any real network calls.

If you've ever wired up a CLI framework before, you've probably seen this pattern: a "hello world" subcommand that exercises the parser and dispatcher without doing anything substantive.

---

## Walkthrough

### Constants

```rust
#[allow(dead_code)]
pub const SUCCESS_MESSAGE: &str = "[Action:Success] Response successful.";
#[allow(dead_code)]
pub const FAIL_MESSAGE: &str = "[Action:Fail] Has failed.";
```

Two `const` strings. Like the `DEFAULT_TITLE` we saw in `deployment_details.rs`, these are inlined by the compiler at every use site.

`#[allow(dead_code)]` suppresses the warning that the constants are technically unused — they're declared for symmetry and in case future code wants to reference them.

The `[Action:Success]` and `[Action:Fail]` prefixes match the PHP original's error/message format exactly.

### `sample_success()`

```rust
pub fn sample_success() -> ApiResponse {
    ApiResponse::ok(json!(SUCCESS_MESSAGE))
}
```

This is the entire implementation. It:

1. Wraps the `SUCCESS_MESSAGE` constant in a `serde_json::json!()` macro call.
2. Passes the resulting JSON value to `ApiResponse::ok(...)`.
3. Returns the `ApiResponse`.

**Teaching points:**

- **`json!(...)` macro** — from `serde_json`. It looks like JSON syntax but runs at compile time (or rather, at runtime, but it produces a `serde_json::Value`). It can contain variables: `json!({ "name": user_name })`.
- **`ApiResponse::ok(...)`** — the constructor we saw in `error.rs`. It produces `{ status: 200, reason: "OK", body: <whatever> }`.

So when you run:

```sh
nautpie deploy:naut sampleSuccess
```

…the program prints:

```json
{"status":200,"reason":"OK","body":"[Action:Success] Response successful."}
```

…and exits with code 0.

### `sample_fail()`

```rust
pub fn sample_fail() -> Result<ApiResponse, Error> {
    Err(Error::Generic(FAIL_MESSAGE.into()))
}
```

Returns `Err(Error::Generic(...))`. The dispatcher in `main.rs` catches this and:

1. Prints the error message as a warning on stderr.
2. Prints a JSON envelope on stdout with `status: 1, reason: "Bad Request", body: <error>`.
3. Exits with code 1.

The return type is `Result<ApiResponse, Error>` rather than just `Error` because Rust forces every function to have a single return type. Returning a `Result` makes it composable with other code that returns `Result`s.

**Teaching points:**

- **`Err(Error::Generic(...))`** — constructs the `Error::Generic` variant (the catch-all error) wrapping the `FAIL_MESSAGE`.
- **`FAIL_MESSAGE.into()`** — `FAIL_MESSAGE` is a `&str`; `Error::Generic` takes a `String`. The `Into` trait handles the conversion.

### How it's wired into `DeployNaut::run()`

In `deploy_naut.rs`, the dispatch is:

```rust
match action.to_ascii_lowercase().as_str() {
    "samplesuccess" => Ok(sample::sample_success()),
    "samplefail" => sample::sample_fail(),
    // ... other actions ...
}
```

Note how `sample_success()` already returns `ApiResponse`, so we wrap it in `Ok(...)`. `sample_fail()` already returns `Result<ApiResponse, Error>`, so we use it as-is.

### Tests

```rust
#[test]
fn success_message_round_trip() {
    let r = sample_success();
    assert_eq!(r.status, 200);
    assert_eq!(r.reason, "OK");
    assert_eq!(r.body, json!(SUCCESS_MESSAGE));
}

#[test]
fn fail_returns_error() {
    let err = sample_fail().unwrap_err();
    assert!(matches!(err, Error::Generic(ref m) if m == FAIL_MESSAGE));
}
```

Two quick tests that verify:

- `sample_success()` produces the expected envelope.
- `sample_fail()` produces the expected error.

**Teaching points:**

- **`matches!(err, Error::Generic(ref m) if m == FAIL_MESSAGE)`** — pattern-match with a guard. `ref m` means "bind a reference to the inner `String` as `m`." The guard `if m == FAIL_MESSAGE` checks the bound value. Note: this works because `Error` derives `PartialEq`, so `m == FAIL_MESSAGE` is allowed.

---

## Why Is It Written This Way?

- **Tiny smoke tests.** The whole point of these actions is that they're tiny. They prove the CLI works without exercising any real API.
- **PHP parity.** The PHP original had `doSampleSuccess` and `doSampleFail` methods on its helper class. The Rust port keeps the same names and the same prefix-tagged messages.
- **No network calls.** Everything happens in memory. CI scripts can run `nautpie deploy:naut sampleSuccess` to verify the binary is callable.
- **Real `Result` plumbing.** Even though these functions are trivial, they exercise the same `Result<ApiResponse, Error>` plumbing as the real actions. So they're a meaningful test of the dispatcher.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub const NAME: &str = "..."` | constants | Compile-time string constant |
| `#[allow(dead_code)]` | constants | Suppress unused-code warnings |
| `json!(...)` macro | `sample_success` | Build a `serde_json::Value` from JSON-like syntax |
| `ApiResponse::ok(...)` | `sample_success` | Construct a success response |
| `Result<T, E>` | `sample_fail` return | Success-or-failure return type |
| `Err(variant(field))` | `sample_fail` body | Construct an error value |
| `.into()` | `FAIL_MESSAGE.into()` | Convert `&str` to `String` |
| `matches!(expr, pat)` | test | Pattern-match an expression in one line |
| `ref` pattern | test | Bind a reference to the inner value |
| Pattern guard `if m == ...` | test | Refine a pattern with an extra condition |
| `unwrap_err()` | test | Assert that a `Result` is `Err` and extract it |
| `assert_eq!(a, b)` | tests | Verify equality, panic with diff otherwise |
