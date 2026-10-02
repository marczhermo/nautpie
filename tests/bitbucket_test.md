# `tests/bitbucket_test.rs` — Wiremock Tests for `ci:bitbucket` Actions

This file contains integration tests for the `ci:bitbucket` actions defined in `src/commands/bitbucket.rs`. Like `http_test.rs`, it uses `wiremock` to spin up real HTTP servers and exercises the actual `ReqwestHttpClient`.

---

## The Big Picture

The tests cover the three `ci:bitbucket` actions:

| Action | Tests |
|---|---|
| `createAccessToken` | success returns the access_token field; 401 returns `Error::HttpStatus`. |
| `createTag` | tag without commit is sent as-is; tag containing commit is truncated; short commit errors. |
| `deployPackage` | full happy path with token + deploy mock; short commit errors. |

The tests also demonstrate a recurring pattern: **environmental state must be carefully managed** because `do_create_access_token` and `do_deploy_package` read environment variables.

---

## Walkthrough

### Imports

```rust
use std::sync::{Mutex, MutexGuard};

use nautpie::commands::bitbucket::{do_create_access_token, do_create_tag, do_deploy_package};
use nautpie::error::{ApiResponse, Error};
use nautpie::http::{HttpClient, ReqwestHttpClient};
use nautpie::io::{Io, StderrIo};
use nautpie::options::Options;
```

- **`std::sync::{Mutex, MutexGuard}`** — for serializing environment-variable mutations across tests.
- **`nautpie::commands::bitbucket::*`** — the three public functions we test.
- **`include_str!`** macro is used at the top to embed JSON fixtures.

### Fixture

```rust
const FIXTURE_CREATE_SUCCESS: &str = include_str!("fixtures/createDeploymentSuccess.json");
```

**`include_str!`** — a macro that includes the file's contents as a `&'static str` at compile time. This means the fixture file is part of the test binary; no runtime file I/O.

### Helper: `start_server()`

Identical to the helper in `http_test.rs`. Briefly:

```rust
fn start_server<F>(setup: F) -> wiremock::MockServer
where
    F: for<'a> std::ops::FnOnce(&'a wiremock::MockServer)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builds");
    let server = rt.block_on(async { wiremock::MockServer::start().await });
    rt.block_on(async { setup(&server).await });
    drop(rt);
    server
}
```

Starts a tokio runtime, starts a mock server, runs the user's setup to mount mocks, then drops the runtime. Returns the running `MockServer`.

### Helper: `client_at()`

```rust
fn client_at(server: &wiremock::MockServer) -> ReqwestHttpClient {
    let mut c = ReqwestHttpClient::new().expect("client builds");
    c.set_endpoint(&server.uri());
    c
}
```

Build a client and point it at the mock server.

### Helper: `opts()`

```rust
fn opts(pairs: &[(&str, &str)]) -> Options {
    let mut o = Options::new();
    for (k, v) in pairs {
        o.set(*k, (*v).to_string());
    }
    o
}
```

Build an `Options` bag from a list of `(key, value)` pairs. Useful for tests that need to construct specific CLI option sets.

**Teaching points:**

- **`pairs: &[(&str, &str)]`** — a borrowed slice of tuples of borrowed string slices.
- **`*k, *v`** — dereference the tuple elements. `k` and `v` are `&&str`; `*k` gives `&str`, which we then pass to `set` via `.to_string()`.

### Env-lock pattern

```rust
struct EnvLock(MutexGuard<'static, ()>);
static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn lock_env() -> EnvLock {
    EnvLock(ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner()))
}

impl Drop for EnvLock {
    fn drop(&mut self) {
        for key in [
            "NAUT_ENDPOINT",
            "DASH_USER",
            "DASH_TOKEN",
            "BB_ENDPOINT",
            "BB_AUTH_STRING",
            "BITBUCKET_REPO_OWNER",
            "BITBUCKET_REPO_SLUG",
            "BITBUCKET_BRANCH",
            "BB_CONSUMER_KEY",
            "BB_CONSUMER_SECRET",
            "BB_OAUTH_ENDPOINT",
        ] {
            std::env::remove_var(key);
        }
    }
}
```

This is a **critical pattern** for any test that mutates process-wide environment variables.

**Teaching points:**

- **`static ENV_MUTEX: Mutex<()> = Mutex::new(());`** — a global mutex. `()` is the "type parameter" — we don't actually care about the protected data, just that only one test mutates env vars at a time.
- **`MutexGuard<'static, ()>`** — the guard returned by `Mutex::lock`. The `'static` lifetime comes from the `static` mutex.
- **`unwrap_or_else(|e| e.into_inner())`** — if the lock is **poisoned** (a thread panicked while holding it), recover by extracting the inner guard anyway. Tests should still run.
- **`impl Drop for EnvLock`** — when the `EnvLock` is dropped (e.g., end of test function), the env vars are cleared. This is **RAII**: the cleanup happens automatically when the lock goes out of scope.

#### `setup_bitbucket_env()`

```rust
fn setup_bitbucket_env(server_uri: &str) -> EnvLock {
    let g = lock_env();
    std::env::set_var("BB_OAUTH_ENDPOINT", server_uri);
    std::env::set_var("BB_ENDPOINT", server_uri);
    std::env::set_var("BB_CONSUMER_KEY", "BB_CONSUMER_KEY");
    std::env::set_var("BB_CONSUMER_SECRET", "BB_CONSUMER_SECRET");
    std::env::set_var("BITBUCKET_REPO_OWNER", "ssmarco");
    std::env::set_var("BITBUCKET_REPO_SLUG", "cd-test");
    std::env::set_var("BITBUCKET_BRANCH", "release/something");
    std::env::set_var("NAUT_ENDPOINT", server_uri);
    std::env::set_var("DASH_USER", "u");
    std::env::set_var("DASH_TOKEN", "t");
    g
}
```

Sets all the env vars that `do_create_access_token`, `do_create_tag`, and `do_deploy_package` need. Returns the `EnvLock` so the caller can hold it for the duration of the test (and cleanup happens at the end).

**Teaching points:**

- **`BB_OAUTH_ENDPOINT` and `BB_ENDPOINT` are set to the same `server_uri`** — both functions read either or both. Setting both is defensive.
- **`BB_CONSUMER_KEY` set to `"BB_CONSUMER_KEY"`** — the value is the same as the variable name, just for clarity.
- **The `let _env = setup_bitbucket_env(...)` pattern** — the test binds the returned `EnvLock` to `_env`. The underscore prefix tells the compiler "I know I'm not using this value; I'm just keeping it alive for its `Drop` side effect."

### Test: `create_access_token_returns_response`

```rust
#[test]
fn create_access_token_returns_response() {
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            let body = serde_json::json!({
                "access_token": "R2uy6EHAKD7K1q3IG9FHf1B4hq9IprTHiLsT0HnA=",
                "scopes": "repository",
                "expires_in": 7200,
                "refresh_token": "Sdyr6UewYGxsmgDH78",
                "token_type": "bearer",
            })
            .to_string();
            Mock::given(method("POST"))
                .and(path("/oauth2/access_token/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .mount(s)
                .await;
        })
    });
    let _env = setup_bitbucket_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[]);
    let resp = do_create_access_token(&options, &mut client).expect("token succeeds");
    assert_eq!(resp.status, 200);
    assert_eq!(
        resp.body["access_token"],
        "R2uy6EHAKD7K1q3IG9FHf1B4hq9IprTHiLsT0HnA="
    );
    assert_eq!(resp.body["scopes"], "repository");
}
```

**Teaching points:**

- **`use ...` inside the closure** — the imports are scoped to just this test. This avoids polluting the file-level imports and makes it clear which test uses which dependency.
- **`opts(&[])`** — empty options. `createAccessToken` reads everything from env vars, not options.
- **`resp.body["access_token"]`** — index into the JSON body (a `serde_json::Value`) by key.

### Test: `create_access_token_non_200_errors`

```rust
let err = do_create_access_token(&options, &mut client).expect_err("401 errors");
assert!(matches!(err, Error::HttpStatus { status: 401, .. }));
```

**Teaching points:**

- **`matches!(err, Error::HttpStatus { status: 401, .. })`** — the `..` is a "rest pattern." It matches any other fields in the struct variant without binding them. So we're asserting that the error is an `HttpStatus` with status 401, ignoring `reason` and `body`.

### Test: `create_tag_truncates_commit_in_tag`

```rust
let options = opts(&[
    ("commit", "COMMIT_HASH_REQUIRES_40_CHARS_1234567890"),
    ("tag", "v1.2.34"),
]);
let resp = do_create_tag(&options, &mut client).expect("createTag succeeds");
assert_eq!(resp.status, 201);
assert_eq!(resp.body["name"], "v1.2.34");
```

The tag (`v1.2.34`) does not contain the commit string, so PHP's `strrpos` returns false. The Rust port's `tag.rfind(&commit)` returns `None`. The tag is sent as-is.

### Test: `create_tag_truncates_when_tag_contains_commit`

```rust
let commit = "COMMIT_HASH_REQUIRES_40_CHARS_1234567890";
let tag = format!("release/{commit}-extra");
let options = opts(&[("commit", commit), ("tag", &tag)]);
```

The tag contains the full commit string. PHP's `strrpos` returns the position; the tag is truncated to `pos + 7` chars. The Rust port replicates this with `tag.rfind(&commit)` and `tag.truncate(pos + cap)`.

**Teaching points:**

- **`format!("release/{commit}-extra")`** — produces something like `release/COMMIT_HASH_REQUIRES_40_CHARS_1234567890-extra`.
- **The mock just returns 201** — we don't directly verify the truncation in the wiremock body. The test asserts the request succeeded; the truncation logic is tested indirectly via the unit tests in `bitbucket.rs` itself.

### Test: `create_tag_rejects_short_commit`

```rust
let _env = setup_bitbucket_env("http://unused");
let mut client = ReqwestHttpClient::new().unwrap();
let options = opts(&[("commit", "abc"), ("tag", "v1")]);
let err = do_create_tag(&options, &mut client).expect_err("short commit errors");
assert!(matches!(err, Error::Generic(_)));
```

**Teaching points:**

- **`setup_bitbucket_env("http://unused")`** — the URI doesn't matter because `do_create_tag` errors before any HTTP call. We just need the env vars set.
- **`Error::Generic(_)`** — the `_` matches any inner data (we don't care which error message it is).

### Test: `deploy_package_returns_single_create_deployment_response`

This is the **biggest** test in the file. It exercises the full `deployPackage` flow:

1. Mock the OAuth token endpoint to return a valid token.
2. Mock the DeployNaut deployment endpoint to return a successful fixture.
3. Call `do_deploy_package` with a 40-char commit, stack, and environment.
4. Verify the response.

```rust
let token_body = serde_json::json!({
    "access_token": "R2uy6EHAKD7K1q3IG9FHf1B4hq9IprTHiLsT0HnA=",
    ...
}).to_string();
let deploy_fixture: serde_json::Value = serde_json::from_str(FIXTURE_CREATE_SUCCESS).unwrap();
let deploy_body = serde_json::to_string(&deploy_fixture["body"]).unwrap();

let server = start_server(move |s| {
    let token_body = token_body.clone();
    let deploy_body = deploy_body.clone();
    Box::pin(async move {
        // mount two mocks: one for the token endpoint, one for the deployment
    })
});
```

**Teaching points:**

- **`serde_json::from_str(FIXTURE_CREATE_SUCCESS).unwrap()`** — parse the fixture JSON file (included at compile time) into a `Value`.
- **`&deploy_fixture["body"]`** — index into the parsed fixture. `deploy_fixture["body"]` is itself a `Value`. We pass `&Value` to `to_string`.
- **`move |s| { ... }`** — the outer closure takes ownership of `token_body` and `deploy_body`.
- **`let token_body = token_body.clone();`** — clone before the inner `move` takes ownership.
- **Two mocks mounted** — one for the OAuth endpoint, one for the deployment endpoint. The deployment flow makes both calls in sequence.

```rust
let mut io = StderrIo::new();
io.set_options(options.clone());
let resp = do_deploy_package(&io.options().unwrap(), &mut io, &mut client)
    .expect("deployPackage succeeds");
```

**Teaching points:**

- **`StderrIo::new()`** — construct a real `Io` impl (not a mock).
- **`io.set_options(options.clone())`** — stuff the options into the Io (the side-channel pattern from `io/mod.rs`).
- **`&io.options().unwrap()`** — pull the options back out and pass them to `do_deploy_package`.
- **`options.clone()`** — `set_options` consumes the `Options`, so we clone before passing in.

```rust
assert_eq!(resp.status, 201);
assert_eq!(resp.reason, "Created");
assert_eq!(resp.body["data"]["attributes"]["ref_type"], "package");
assert!(resp.body["data"]["attributes"]["package_link"]
    .as_str()
    .unwrap_or_default()
    .starts_with("https://"));
```

**Teaching points:**

- **`resp.body["data"]["attributes"]["ref_type"]`** — chained JSON indexing. Each `[]` returns a `Value`. We assert it's `"package"` (because the override sets `ref_type` to `"package"`).
- **`.as_str().unwrap_or_default()`** — try to extract a string; default to empty if the field is missing or not a string.
- **`.starts_with("https://")`** — substring check at the start.

### Test: `deploy_package_rejects_short_commit`

Same pattern as the create_tag short-commit test: set env vars to a dummy URI, call with a short commit, expect `Error::Generic`.

### Tail utility

```rust
#[allow(dead_code)]
fn _serde_used() {
    let _: Option<ApiResponse> = None;
}
```

A small helper that exists only to keep the `serde_json` and `ApiResponse` imports alive (so clippy doesn't complain about unused imports). Common pattern at the bottom of test files.

---

## Why Is It Written This Way?

- **Real HTTP via wiremock.** Same philosophy as `http_test.rs`: catch issues that pure mocks would miss.
- **Env-lock pattern.** Process-wide env var mutations are dangerous in parallel test runs. The mutex serializes them, and the `Drop` impl cleans up after each test.
- **Fixture files via `include_str!`.** The JSON fixtures (in `tests/fixtures/`) are unchanged from the PHP repo. They live alongside the tests, not in source code.
- **Test names describe the scenario.** `create_tag_truncates_when_tag_contains_commit` immediately tells you what scenario is being tested. The Rust convention for test names is `snake_case`, often describing the behavior under test.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `tests/` directory | whole file | Integration tests live here |
| `#[test]` | every test | Marks a function as a test |
| `include_str!` macro | fixture loading | Include a file's contents as `&'static str` |
| `static MUTEX: Mutex<T> = Mutex::new(...)` | `ENV_MUTEX` | Global mutex for serializing access |
| `MutexGuard<'static, ()>` | `EnvLock` | The guard type from `Mutex::lock` |
| `unwrap_or_else(\|e\| e.into_inner())` | `lock_env` | Recover from a poisoned mutex |
| `impl Drop for EnvLock` | bottom | RAII cleanup when the lock goes out of scope |
| `for<'a> FnOnce(&'a ...) -> Pin<Box<dyn Future + 'a>>` | `start_server` | HRTB for the async setup closure |
| `tokio::runtime::Builder` | `start_server` | Build a tokio runtime |
| `Pin<Box<dyn Future>>` | setup closures | Pinned, boxed, type-erased future |
| `Box::pin(async move { ... })` | setup closures | Pin a future on the heap |
| `move` closure | several tests | Move captured vars into the closure |
| `.clone()` inside `move` closure | several tests | Clone outer vars for the inner closure |
| `use ...` inside a closure | several tests | Localized imports |
| `serde_json::from_str(...)` | fixture loading | Parse a string as JSON |
| `serde_json::to_string(&value)` | deploy test | Serialize a `Value` to a string |
| `serde_json::json!({...})` | several tests | Build a `Value` from JSON-like syntax |
| `to_string()` | multiple | Convert values to owned strings |
| `from_str(FIXTURE).unwrap()` | several | Parse and panic-on-fail |
| `matches!(err, Error::Variant { field: x, .. })` | several | Pattern-match with rest-binding `..` |
| `_env` binding | every setup | Hold the env lock for its `Drop` side effect |
| `options.clone()` | deploy test | Clone the options before passing to `set_options` |
| `Value` indexing `["key"]` | assertions | Index into a JSON value |
| `.as_str().unwrap_or_default()` | assertion | Try to extract a string, default to "" |
| `.starts_with(...)` | assertion | Check a prefix |
| `#[allow(dead_code)]` | tail | Suppress unused-code warnings |
