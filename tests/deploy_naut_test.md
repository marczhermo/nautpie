# `tests/deploy_naut_test.rs` — Wiremock Tests for `deploy:naut` Actions

This file contains integration tests for the `deploy:naut` actions. It's the **largest** of the test files (~490 lines) because `deploy:naut` has the most actions and the most complex behaviors.

---

## The Big Picture

The tests cover:

| Action | Tests |
|---|---|
| `fetch` | success returns the meta envelope; 400 returns `Error::HttpStatus`. |
| `createDeployment` | success; 400; missing ref; with overrides; prod ignores bypass_and_start. |
| `getDeployments` | success returns sorted data. |
| `lastDeployment` | returns the first record (sorted desc). |
| `gitFetch` | success returns final completed body; non-202 errors. |
| `checkDeploymentProgress` | polls until `Completed`. |

The file also defines a `CaptureIo` struct — a custom `Io` impl used in some tests.

---

## Walkthrough

### Imports

```rust
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use serde_json::json;

use nautpie::commands::deploy_naut::{
    do_check_deployment_progress, do_create_deployment, do_fetch, do_get_deployments,
    do_git_fetch, do_last_deployment, DeploymentOverrides,
};
use nautpie::error::{ApiResponse, Error};
use nautpie::http::{HttpClient, ReqwestHttpClient};
use nautpie::io::{Io, StderrIo};
use nautpie::options::Options;
```

The same imports as `bitbucket_test.rs`, plus the `deploy_naut` actions and `serde_json::json`.

### Fixtures

```rust
const FIXTURE_GET_DEPLOYMENTS: &str = include_str!("fixtures/getDeployments.json");
const FIXTURE_CREATE_SUCCESS: &str = include_str!("fixtures/createDeploymentSuccess.json");
const FIXTURE_CREATE_ERROR: &str = include_str!("fixtures/createDeploymentError.json");
```

Three JSON files embedded at compile time. They are unchanged from the PHP repo.

### Helpers

The helpers (`start_server`, `client_at`, `opts`, `lock_env`, `EnvLock`, `setup_deploy_env`) are essentially the same as in `bitbucket_test.rs`. The only difference is `setup_deploy_env`:

```rust
fn setup_deploy_env(server_uri: &str) -> EnvLock {
    let g = lock_env();
    std::env::set_var("NAUT_ENDPOINT", server_uri);
    std::env::set_var("DASH_USER", "marco");
    std::env::set_var("DASH_TOKEN", "token123");
    g
}
```

`deploy:naut` actions only need `NAUT_ENDPOINT`, `DASH_USER`, and `DASH_TOKEN`. We don't set the Bitbucket-specific env vars.

**Teaching points:**

- **`"Bare host:port (no path component)"` comment in the source** — the comment explains that the wiremock server's URI has no path. So request URLs are rooted at `/` and match mock paths directly. This is a subtle gotcha: if the URI had a path like `/naut`, the request URL would be `/naut/project/...` and the mocks wouldn't match.

### `CaptureIo`

```rust
struct CaptureIo {
    opts: Option<Options>,
}

impl CaptureIo {
    fn new(opts: Options) -> Self {
        Self { opts: Some(opts) }
    }
}

impl Io for CaptureIo {
    fn warning(&mut self, _: &str) {}
    fn success(&mut self, _: &str) {}
    fn message(&mut self, _: &str) {}
    fn write_json(&mut self, _: &ApiResponse) {}
    fn set_options(&mut self, opts: Options) {
        self.opts = Some(opts);
    }
    fn options(&self) -> Option<Options> {
        self.opts.clone()
    }
}

#[allow(dead_code)]
fn _capture_helper_compiles() {
    let o = CaptureIo::new(Options::new());
    let _: &dyn Io = &o;
}
```

A custom `Io` implementation used to verify that the trait compiles correctly.

**Teaching points:**

- **`fn _capture_helper_compiles()`** — a function that exists only to assert the code type-checks. The `let _: &dyn Io = &o;` line forces the compiler to verify that `CaptureIo` is a valid `Io` impl.
- **`#[allow(dead_code)]`** — silences the warning that nothing else uses this function.

This is a common Rust pattern: write a "compile-only" test to ensure your trait implementations are correct without writing a full behavioral test.

### Test: `fetch_returns_meta_envelope`

```rust
#[test]
fn fetch_returns_meta_envelope() {
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            let body = serde_json::json!({"meta": {"whoami": "marco@example.com", "now": "2017-05-09 11:57:00"}}).to_string();
            Mock::given(method("GET"))
                .and(path("/meta"))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .mount(s)
                .await;
        })
    });
    let mut client = client_at(&server);
    let options = opts(&[("url", "meta")]);
    let resp = do_fetch(&options, &mut client).expect("fetch succeeds");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.reason, "OK");
    assert_eq!(resp.body["meta"]["whoami"], "marco@example.com");
}
```

A standard "happy path" test for `fetch`. The mock returns a small JSON object; the test verifies the response status, reason, and a specific field of the parsed body.

### Test: `fetch_bad_response_propagates_error`

```rust
let err = do_fetch(&options, &mut client).expect_err("400 should error");
match err {
    Error::HttpStatus { status, reason, body } => {
        assert_eq!(status, 400);
        assert_eq!(reason, "Bad Request");
        assert!(body.contains("not supported"));
    }
    other => panic!("expected HttpStatus, got {other:?}"),
}
```

Verifies that a 400 response from the server becomes `Error::HttpStatus`. The `match` extracts `status`, `reason`, and `body` for assertions.

### Test: `missing_action_returns_error_envelope`

```rust
let server = start_server(|_| Box::pin(async {}));
let mut client = client_at(&server);
let mut io = StderrIo::new();
io.set_options(opts(&[]));
let err = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
    .expect_err("empty opts should error");
assert!(matches!(err, Error::MissingOption(_)));
```

Empty options bag → `MissingOption`. Note the empty `start_server` setup — no mocks are mounted because the request errors before any HTTP call.

### Test: `get_deployments_returns_sorted_data`

```rust
let server = start_server(|s| {
    let body = FIXTURE_GET_DEPLOYMENTS.to_string();
    Box::pin(async move {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        Mock::given(method("GET"))
            .and(path("/project/stack/environment/uat/deploys"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(s)
            .await;
    })
});
let _env = setup_deploy_env(&server.uri());
let mut client = client_at(&server);
let options = opts(&[("stack", "stack"), ("environment", "uat")]);
let resp = do_get_deployments(&options, &mut client).expect("list succeeds");
assert_eq!(resp.status, 200);
let data = resp.body["data"].as_array().expect("data array");
assert_eq!(data.len(), 2);
assert_eq!(data[0]["id"], "64989");
assert_eq!(data[1]["id"], "64962");
```

**Teaching points:**

- **`FIXTURE_GET_DEPLOYMENTS.to_string()`** — convert the `&'static str` to a `String` (the mock expects a `String`).
- **`resp.body["data"].as_array().expect("data array")`** — downcast the `data` field to a JSON array. `expect` panics if it's not an array.
- **`data[0]["id"]`** — index into the array, then into the object. The result is a `Value` we compare with `assert_eq!`.
- **The sort assertion** — the IDs are 64989 then 64962, which is descending. The action sorts deployments by ID descending, so the first item should have the highest ID.

### Test: `last_deployment_returns_first_record`

```rust
let original: serde_json::Value = serde_json::from_str(FIXTURE_GET_DEPLOYMENTS).unwrap();
assert_eq!(resp.body, original["data"][0]);
```

Verifies that `last_deployment` returns the same JSON object as the first item in the `data` array of the fixture.

### Test: `create_deployment_success`

```rust
let fixture: serde_json::Value = serde_json::from_str(FIXTURE_CREATE_SUCCESS).unwrap();
let inner_body = serde_json::to_string(&fixture["body"]).unwrap();
let server = start_server(move |s| {
    let inner_body = inner_body.clone();
    Box::pin(async move {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        Mock::given(method("POST"))
            .and(path("/project/stack/environment/uat/deploys"))
            .respond_with(ResponseTemplate::new(201).set_body_string(inner_body))
            .mount(s)
            .await;
    })
});
let _env = setup_deploy_env(&server.uri());
let mut client = client_at(&server);
let options = opts(&[
    ("stack", "stack"),
    ("environment", "uat"),
    ("ref_type", "branch"),
    ("ref", "develop"),
]);
let mut io = StderrIo::new();
io.set_options(options.clone());
let resp = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
    .expect("create succeeds");
assert_eq!(resp.status, 201);
assert_eq!(resp.reason, "Created");
assert_eq!(resp.body, fixture["body"]);
```

A round-trip test: send a real `createDeployment`, mock returns a fixture, verify the response body equals the fixture body.

**Teaching points:**

- **`fixture["body"]`** — the inner "body" object of the fixture (the envelope has `{status, reason, body}`).
- **`options = opts(&[("stack", "stack"), ..., ("ref", "develop")])`** — supply all required options.
- **`assert_eq!(resp.body, fixture["body"])`** — compare two `Value`s. They implement `PartialEq` so this works.

### Test: `create_deployment_gone_bad`

```rust
let err = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
    .expect_err("400 errors");
match err {
    Error::HttpStatus { status, reason, .. } => {
        assert_eq!(status, 400);
        assert_eq!(reason, "Bad Request");
    }
    other => panic!("expected HttpStatus, got {other:?}"),
}
```

**Teaching points:**

- **`Error::HttpStatus { status, reason, .. }`** — destructures only `status` and `reason`. The `..` ignores `body`.

### Test: `create_deployment_missing_ref_errors`

```rust
let options = opts(&[
    ("stack", "stack"),
    ("environment", "uat"),
    ("ref_type", "branch"),
]);
// note: no "ref"
let err = do_create_deployment(...).expect_err("missing ref");
match err {
    Error::Generic(m) => assert!(m.contains("Requires ref")),
    other => panic!(...),
}
```

The action explicitly errors with `"[Action:CreateDeployment] Requires ref option"` when no ref is supplied (and not redeploying). The test verifies the error message contains "Requires ref".

### Test: `create_deployment_with_overrides_succeeds`

```rust
let overrides = DeploymentOverrides {
    ref_: Some("https://example.com/pkg.tar.gz".into()),
    ref_type: Some("package".into()),
    title: Some("[CD:Package] abc".into()),
    summary: Some("Branch:main".into()),
};
let mut io = StderrIo::new();
io.set_options(options.clone());
let resp = do_create_deployment(
    &io.options().unwrap(),
    &mut io,
    &mut client,
    Some(overrides),
)
.expect("create with overrides succeeds");
assert_eq!(resp.status, 201);
```

Verifies that `DeploymentOverrides` is correctly applied — the action uses the override values instead of what's in the options bag.

**Teaching points:**

- **`ref_: Some(...)`** — struct field with trailing underscore because `ref` is a reserved word.
- **`Some(overrides)`** — wrap in `Option::Some` because the parameter type is `Option<DeploymentOverrides>`.

### Test: `create_deployment_prod_environment_ignores_bypass_and_start`

This is the **most subtle** test in the file.

```rust
use wiremock::matchers::body_partial_json;
...
Mock::given(method("POST"))
    .and(path("/project/stack/environment/prod/deploys"))
    .and(body_partial_json(json!({"bypass_and_start": false})))
    .respond_with(ResponseTemplate::new(201).set_body_string(inner_body))
    .mount(s)
    .await;
```

The action has a guard: `if bypass_and_start && !is_production(&environment) { ... }`. So when the environment is `"prod"`, the `bypass_and_start` flag in the payload should be `false` even if the user passed `--bypass_and_start=true`.

**Teaching points:**

- **`body_partial_json(...)`** — a wiremock matcher that checks the request body **partially matches** the given JSON. We don't need to match the entire body; we just need to verify that `bypass_and_start` is `false`.
- **`json!({"bypass_and_start": false})`** — the partial match.
- **The mock path is `/project/stack/environment/prod/deploys`** — different from the uat path. The mock will only match if the production URL is hit.

### Test: `git_fetch_returns_final_completed_body`

```rust
Mock::given(method("POST"))
    .and(path("/project/example/git/fetches"))
    .respond_with(ResponseTemplate::new(202).set_body_string(r#"{"data":{"id":42}}"#))
    .mount(s)
    .await;
Mock::given(method("GET"))
    .and(path("/project/example/git/fetches/42"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_string(r#"{"data":{"attributes":{"status":"Complete"}}}"#),
    )
    .mount(s)
    .await;
```

Two mocks: the POST returns 202 with an ID, the GET returns the final status. The action should call both and return the final state.

**Teaching points:**

- **Two endpoints, one test** — the action makes a POST then polls with GETs. We mock both endpoints.
- **`/git/fetches/42`** — the GET path includes the ID from the POST response. This is the **dynamic URL** pattern: the second URL is constructed from data returned by the first call.

### Test: `git_fetch_non_202_returns_error`

```rust
Mock::given(method("POST"))
    .and(path("/project/example/git/fetches"))
    .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
    .mount(s)
    .await;
...
let err = do_git_fetch(&options, &mut client).expect_err("500 errors");
assert!(matches!(err, Error::HttpStatus { status: 500, .. }));
```

If the POST returns something other than 202, the action errors immediately (rather than trying to poll).

### Test: `check_deployment_progress_polls_until_completed`

```rust
Mock::given(method("GET"))
    .and(path("/project/stack/environment/uat/deploys/42"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_string(r#"{"data":{"attributes":{"state":"Completed"}}}"#),
    )
    .mount(s)
    .await;
...
let resp = do_check_deployment_progress(&io.options().unwrap(), &mut client)
    .expect("progress check succeeds");
assert_eq!(resp.status, 200);
```

The mock returns `Completed` immediately. The action polls once, sees `Completed`, and returns success.

### Tail helpers

```rust
#[allow(dead_code)]
fn _imports_used() {
    let _h: HashMap<String, String> = HashMap::new();
    let _b: DeploymentOverrides = DeploymentOverrides::default();
}

#[allow(dead_code)]
fn _serde_imported() {
    let _: Option<serde_json::Value> = None;
}
```

Three small "compile-only" helpers that ensure all imports are referenced somewhere (so clippy doesn't complain). Common pattern at the end of test files.

---

## Why Is It Written This Way?

- **Same env-lock pattern as `bitbucket_test.rs`.** Process-wide state must be managed carefully across parallel tests.
- **One test per scenario.** Each test sets up the mocks it needs and asserts on the specific behavior. This makes failures easy to diagnose.
- **Real wiremock servers.** Catches HTTP-level bugs that pure mocks would miss.
- **Compile-only helpers.** A small cost to keep imports referenced and clippy happy.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `tests/` directory | whole file | Integration tests live here |
| `#[test]` | every test | Marks a function as a test |
| `include_str!` macro | fixtures | Include a file as `&'static str` |
| `static MUTEX: Mutex<T>` | env locking | Global mutex for serializing access |
| `MutexGuard<'static, ()>` | `EnvLock` | Guard from `Mutex::lock` |
| `impl Drop for EnvLock` | bottom | RAII cleanup on scope exit |
| `for<'a>` HRTB | `start_server` | Higher-ranked trait bound for async closures |
| `Pin<Box<dyn Future + 'a>>` | `start_server` | Pinned, heap-allocated future |
| `Box::pin(async move { ... })` | setup closures | Pin a future on the heap |
| `tokio::runtime::Builder` | `start_server` | Build a tokio runtime |
| `rt.block_on(...)` | `start_server` | Run an async block to completion |
| `move` closure | several tests | Move captured vars into the closure |
| `.clone()` inside `move` closure | several tests | Clone outer vars for the inner closure |
| `Mock::given(method(...))` | every test | Start a request matcher chain |
| `.and(matcher)` | every test | Compose matchers with logical AND |
| `.respond_with(template)` | every test | Specify the mock response |
| `.mount(server)` | every test | Register the mock |
| `body_partial_json(...)` | prod-ignores test | Match request body partially |
| `body_string(...)` | several tests | Match request body exactly |
| `header("name", "value")` | several tests | Match a request header |
| `method("GET")` | every test | Match by HTTP method |
| `path("/foo")` | every test | Match by URL path |
| `serde_json::json!({...})` | several tests | Build a `Value` from JSON-like syntax |
| `serde_json::from_str(...)` | several | Parse a string as JSON |
| `serde_json::to_string(&value)` | several | Serialize a `Value` to a string |
| `.as_array().expect("data array")` | `get_deployments` test | Downcast a `Value` to an array |
| `Value` indexing `["key"]` | assertions | Index into a JSON value |
| `matches!(err, Variant)` | many | Pattern-match on an enum value |
| `..` in pattern | several | "Bind these fields, ignore the rest" |
| `_env` binding | every setup | Hold the env lock for its `Drop` side effect |
| `let _: Type = ...` | helpers | Type annotation only |
| `#[allow(dead_code)]` | helpers | Suppress unused-code warnings |
| Custom `Io` impl | `CaptureIo` | Implement the trait for testing |
| `&dyn Io` | helper | Trait object reference |
