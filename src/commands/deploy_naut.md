# `src/commands/deploy_naut.rs` — The DeployNaut API Actions

This file is the **largest in the project** — about 350 lines. It implements the `deploy:naut <action>` subcommand, which talks to DeployNaut's REST API.

If `cli.rs` is the **front door** (parsing what the user typed) and `main.rs` is the **conductor** (deciding what to run), this file is the **stage** where most of the actual work happens.

---

## The Big Picture

`deploy:naut` supports these actions:

| Action | What it does |
|---|---|
| `sampleSuccess` | Returns a fixed success message. No network. |
| `sampleFail` | Returns a fixed failure. No network. |
| `fetch` | GETs a path under the API endpoint. |
| `createDeployment` | POSTs a new deployment payload. |
| `gitFetch` | Asks DeployNaut to refresh its git cache, then polls until done. |
| `getDeployments` | Lists recent deployments, optionally filtered by commit. |
| `lastDeployment` | Returns the most recent deployment. |
| `checkDeploymentProgress` | Polls a single deployment until it completes. |

Every action goes through:

1. `setup_deploy_naut_env(http)` — verify the required env vars are set and configure the HTTP client.
2. Build a URL or payload.
3. Call `http.request(...)`.
4. Decode the JSON response and return an `ApiResponse`.

---

## Walkthrough

### Constants

```rust
pub const DEPLOY_TIMEOUT_SECS: u64 = 1800;  // 30 minutes
pub const GIT_TIMEOUT_SECS: u64 = 120;      // 2 minutes
```

Two `u64` (unsigned 64-bit integer) constants. They're `u64` because the `Duration::from_secs(...)` constructor takes a `u64`. These are the **deadlines** for polling operations: 30 minutes for a deployment, 2 minutes for a git fetch.

`pub` means they're visible to other modules. The integration tests use them to shorten the deadline for faster test runs.

### The `DeployNaut` struct and `Command` impl

```rust
pub struct DeployNaut;

impl Command for DeployNaut {
    fn name(&self) -> &'static str { "deploy:naut" }

    fn run(&self, action: &str, io: &mut dyn Io, http: &mut dyn HttpClient)
        -> Result<ApiResponse, Error>
    {
        let opts = io.options()
            .ok_or_else(|| Error::MissingOption("options".into()))?;
        match action.to_ascii_lowercase().as_str() {
            "samplesuccess" => Ok(sample::sample_success()),
            "samplefail"    => sample::sample_fail(),
            "fetch"         => do_fetch(&opts, http),
            "createdeployment" => do_create_deployment(&opts, io, http, None),
            "gitfetch"      => do_git_fetch(&opts, http),
            "getdeployments" => do_get_deployments(&opts, http),
            "lastdeployment" => do_last_deployment(&opts, http),
            "checkdeploymentprogress" => do_check_deployment_progress(&opts, http),
            _ => Err(Error::MissingAction),
        }
    }
}
```

**Teaching points:**

- **`io.options().ok_or_else(|| Error::MissingOption("options".into()))?`** — three Rust idioms in one line:
  - `io.options()` returns `Option<Options>`.
  - `.ok_or_else(|| ...)` converts it to `Result<Options, Error>`. The closure form (`ok_or_else`) is preferred over `ok_or` when constructing the error is non-trivial, because the closure only runs on `None`.
  - `?` returns early with the error if it's `Err`.
- **`action.to_ascii_lowercase().as_str()`** — converts the action string to lowercase ASCII, then borrows it as `&str` so we can pattern-match in the `match`. We could also use `eq_ignore_ascii_case` for each arm, but lowercase-then-match is cleaner.
- **Each arm returns a `Result<ApiResponse, Error>`** but with different shapes:
  - `sample_success()` returns `ApiResponse` directly, so we wrap in `Ok(...)`.
  - `sample_fail()` already returns `Result<ApiResponse, Error>`.
  - `do_fetch(...)` etc. return `Result<ApiResponse, Error>` directly.

### `options_from_deploy_args()`

```rust
pub fn options_from_deploy_args(args: &DeployNautArgs) -> Options {
    let mut opts = Options::new();
    if let Some(v) = &args.url { opts.set("url", v.clone()); }
    if let Some(v) = &args.commit { opts.set("commit", v.clone()); }
    // ... etc ...
    opts
}
```

This converts a `DeployNautArgs` (from `cli.rs`) into an `Options` bag. It's called once in `main.rs` after `clap` parses the args.

**Teaching points:**

- **`if let Some(v) = &args.url`** — pattern-match only the `Some` variant. If `args.url` is `None`, the block is skipped silently.
- **`v.clone()`** — we need an owned `String` to put into the `Options` hash map; `args.url` is `Option<String>` and we're borrowing it as `&String`, so we have to clone.
- **Repeated `if let`** — verbose but explicit. The alternative (a helper macro or iterator chain) would be cleverer but harder to read.

There's also `options_from_bitbucket_args()` which does the same thing for the Bitbucket subcommand.

### `setup_deploy_naut_env()`

```rust
pub fn setup_deploy_naut_env(http: &mut dyn HttpClient)
    -> Result<(String, String, String), Error>
{
    let endpoint = std::env::var("NAUT_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(Error::MissingEndpoint)?;
    http.set_endpoint(&endpoint);
    http.set_content_type("application/json");
    let creds = check_envs(&["DASH_USER", "DASH_TOKEN"])?;
    let dash_user = creds[0].clone();
    let dash_token = creds[1].clone();
    http.set_username_and_password(&dash_user, &dash_token);
    Ok((endpoint, dash_user, dash_token))
}
```

This is the **shared setup** for every action that talks to DeployNaut. It:

1. Reads `NAUT_ENDPOINT` (returns `MissingEndpoint` if unset or empty).
2. Tells the HTTP client what the base endpoint is.
3. Resets the content type to `application/json`.
4. Reads `DASH_USER` and `DASH_TOKEN` (returns `MissingEnv` if either is unset).
5. Tells the HTTP client what the basic-auth credentials are.
6. Returns the three values so callers don't need to re-read them.

**Teaching points:**

- **`std::env::var("NAUT_ENDPOINT").ok()`** — convert `Result<String, VarError>` to `Option<String>`. `.ok()` discards the error info, which is fine here.
- **`.filter(|v| !v.is_empty())`** — if the value is an empty string, treat it as `None`. This matches `check_envs` semantics.
- **`.ok_or(Error::MissingEndpoint)?`** — convert `Option<String>` to `Result<String, Error>`, then return early on `None`. This is the same pattern as `io.options().ok_or_else(...)` above.
- **`let creds = check_envs(...)?`** — the `?` operator propagates the error. If `DASH_USER` or `DASH_TOKEN` is missing, we bail out.
- **Returning a 3-tuple** — `Result<(String, String, String), Error>`. Callers destructure: `let (endpoint, user, token) = setup_deploy_naut_env(http)?;`.

### `do_fetch()` — the simplest real action

```rust
pub fn do_fetch(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let url = opts.option("url").ok_or_else(|| Error::MissingOption("url".into()))?;
    let resp = http.request("GET", &url, None, None)?;
    let body: Value = serde_json::from_str(&resp.body)
        .map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse {
        status: resp.status,
        reason: resp.reason,
        body,
    })
}
```

The pattern is repeated in every action:

1. Read a required option, return `MissingOption` if absent.
2. Call `http.request(...)`.
3. Decode the JSON body.
4. Construct an `ApiResponse` from the status, reason, and body.

**Teaching points:**

- **`serde_json::from_str(&resp.body)`** — parse the response body as JSON into a `Value`. Returns `Result<Value, serde_json::Error>`.
- **`.map_err(|e| Error::Json(format!("decode failed: {e}")))`** — convert the parse error into our `Error::Json` variant. The closure captures `e` and uses `format!` to build the message.
- **Returning a struct literal** — `Ok(ApiResponse { status: ..., reason: ..., body })`. This is the same as calling a constructor but explicitly named fields. It works because the struct's fields are all `pub`.

### `do_create_deployment()` — the most complex action

This is the heart of the `deploy:naut` subcommand. It builds a `DeploymentDetails` payload, optionally applies some overrides (used by `do_deploy_package` in `bitbucket.rs`), POSTs it, and optionally polls for completion.

```rust
pub fn do_create_deployment(
    opts: &Options,
    io: &mut dyn Io,
    http: &mut dyn HttpClient,
    overrides: Option<DeploymentOverrides>,
) -> Result<ApiResponse, Error> {
    setup_deploy_naut_env(http)?;
    let stack = opts.option("stack").ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts.option("environment").ok_or_else(|| Error::MissingOption("environment".into()))?;
    // ... build details ...
    let relative_url = format!("project/{stack}/environment/{environment}/deploys");
    let body_str = serde_json::to_string(&details.values())
        .map_err(|e| Error::Json(format!("encode failed: {e}")))?;
    let response = http.request("POST", &relative_url, None, Some(body_str))?;
    // ... handle should_wait ...
}
```

**Teaching points:**

- **`Option<DeploymentOverrides>`** — the action accepts **either** options from the CLI **or** explicit overrides (used when `do_deploy_package` synthesises ref/title/summary from the Bitbucket download URL).
- **The override chain** — for each field, the code first checks `overrides`, then falls back to `opts`:
  ```rust
  if let Some(r) = overrides.as_ref().and_then(|o| o.ref_.clone()) {
      details = details.ref_(r);
  } else if let Some(r) = opts.option("ref") {
      details = details.ref_(r);
  }
  ```
  This is verbose but explicit. The alternative is to merge the two sources into one struct first.
- **`.and_then(|o| o.ref_.clone())`** — `overrides.as_ref()` is `Option<&DeploymentOverrides>`. `.and_then(|o| ...)` is `Option::and_then`, which is like `map` but the closure returns another `Option`. The closure clones the inner `Option<String>` to convert from `&Option<String>` to `Option<String>`. So if `overrides` is `None`, we get `None`; if `overrides` is `Some(o)` but `o.ref_` is `None`, we get `None`; if both are `Some`, we get `Some(value)`.
- **`format!("project/{stack}/environment/{environment}/deploys")`** — inline variable interpolation in a format string. Equivalent to `format!("project/{}/environment/{}/deploys", stack, environment)` but more concise.
- **`Some(body_str)`** — pass the serialized JSON as the request body. The HTTP client knows to send this as the request body because it's `Some`.

### `do_git_fetch()` and the polling pattern

```rust
pub fn do_git_fetch(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let stack = opts.option("stack").ok_or_else(|| Error::MissingOption("stack".into()))?;
    setup_deploy_naut_env(http)?;
    let relative_url = format!("project/{stack}/git/fetches");
    let response = http.request("POST", &relative_url, None, None)?;

    if response.status != 202 {
        return Err(Error::HttpStatus { /* ... */ });
    }

    let body = decode_body(&response.body)?;
    let fetch_id = body.get("data").and_then(|d| d.get("id")).and_then(|i| i.as_i64())
        .ok_or_else(|| Error::Generic("[Error:GitFetch] missing data.id".into()))?;

    let poll_url = format!("project/{stack}/git/fetches/{fetch_id}");
    let poll_url_for_closure = poll_url.clone();
    let label = format!("Git fetch for {stack} (#{fetch_id})");
    run_with_progress(
        "gitFetch",
        label,
        Duration::from_secs(GIT_TIMEOUT_SECS),
        http,
        move |http| poll_status(http, &poll_url_for_closure, "status"),
    )?;
    // ... final GET ...
}
```

This is the **two-step polling pattern**: trigger an async operation, then poll until it completes.

**Teaching points:**

- **HTTP 202 (Accepted)** — DeployNaut's git-fetch endpoint returns 202 with a job ID. We poll a separate URL until the job is done.
- **`.get("data").and_then(|d| d.get("id"))`** — chained `Option::and_then` for nested JSON lookup. This is the standard idiom for "navigate a JSON tree without panicking."
- **`poll_url_for_closure = poll_url.clone()`** — the `move` closure needs to own its captured variables. We clone the URL so the outer `poll_url` is still usable after the closure moves in.
- **`move |http| poll_status(http, &poll_url_for_closure, "status")`** — a **move closure**: the `move` keyword moves the captured variables (`poll_url_for_closure`) into the closure. Without `move`, the closure would borrow them, but closures that outlive the calling function (which polling closures do) must own their data.
- **`run_with_progress(...)`** — see `tui/spinner.rs`. It runs the polling loop with a progress UI.

### `do_get_deployments()`

```rust
pub fn do_get_deployments(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let stack = opts.option("stack").ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts.option("environment").ok_or_else(|| Error::MissingOption("environment".into()))?;
    let start_date = opts.option("startDate").unwrap_or_else(|| "-1 year".into());

    setup_deploy_naut_env(http)?;
    let response = fetch_deployments(http, &stack, &environment, &start_date)?;

    let mut deployments = response.body.get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    if let Some(commit) = opts.option("commit") {
        let commit_key = if commit.len() == 7 { "short_sha" } else { "sha" };
        let accepted_states = ["New", "Submitted", "Approved", "Queued", "Deploying", "Completed"];
        deployments.retain(|item| {
            item.get("attributes")
                .and_then(|a| a.get(commit_key))
                .and_then(|v| v.as_str())
                == Some(commit.as_str())
                && item.get("attributes")
                    .and_then(|a| a.get("state"))
                    .and_then(|v| v.as_str())
                    .map(|s| accepted_states.contains(&s))
                    .unwrap_or(false)
        });
    }

    deployments.sort_by(|a, b| { /* ... */ });

    Ok(ApiResponse { status: response.status, reason: response.reason, body: json!({ "data": deployments }) })
}
```

This action lists deployments from the past N days and optionally filters them by commit SHA.

**Teaching points:**

- **`opts.option("startDate").unwrap_or_else(|| "-1 year".into())`** — if the user didn't supply `--startDate`, default to the string `"-1 year"`. The `unwrap_or_else` form is used because the default requires allocation (the `.into()`).
- **`.cloned().unwrap_or_default()`** — three operations chained:
  - `.get("data").and_then(|d| d.as_array())` returns `Option<&Vec<Value>>`.
  - `.cloned()` converts to `Option<Vec<Value>>`.
  - `.unwrap_or_default()` gives us `Vec<Value>`, defaulting to an empty vector if the key was missing.
- **`deployments.retain(|item| ...)`** — like `Vec::retain` in many languages: keep only the items where the closure returns `true`.
- **The `commit_key` ternary** — if the commit is 7 chars long, it's a "short SHA" (the truncated form some platforms show); otherwise it's a full 40-char SHA. DeployNaut stores both.
- **`accepted_states.contains(&s)`** — array `.contains` checks for membership. Returns `true` if `s` is one of the listed states.

### `do_last_deployment()` — composition over duplication

```rust
pub fn do_last_deployment(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let mut response = do_get_deployments(opts, http)?;
    let first = response.body
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first().cloned())
        .unwrap_or(Value::Null);
    response.body = first;
    Ok(response)
}
```

This is beautifully small. It calls `do_get_deployments` (which already sorts newest-first) and grabs the first item.

**Teaching points:**

- **`arr.first().cloned()`** — `first()` returns `Option<&Value>`. `.cloned()` makes it `Option<Value>`. We need an owned `Value` so we can move it into `response.body`.
- **`.unwrap_or(Value::Null)`** — if the array is empty, default to JSON `null`. This is the cleanest "missing data" representation in JSON.
- **Composition over duplication.** Rather than reimplementing the listing + sorting logic, this function **delegates** to `do_get_deployments` and trims the result. This is the **single source of truth** principle.

### `do_check_deployment_progress()`

```rust
pub fn do_check_deployment_progress(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let stack = opts.option("stack").ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts.option("environment").ok_or_else(|| Error::MissingOption("environment".into()))?;
    let deploy_id_str = opts.option("deploy_id").ok_or_else(|| Error::MissingOption("deploy_id".into()))?;
    let deploy_id: i64 = deploy_id_str.parse()
        .map_err(|_| Error::Generic("[Required:Option] deploy_id must be an integer".into()))?;
    check_deployment_progress(deploy_id, &stack, &environment, &mut StderrShim, http)?;
    Ok(ApiResponse::ok(json!("Deployment progress check complete")))
}
```

**Teaching points:**

- **`deploy_id_str.parse()`** — parse the string into `i64`. The turbofish isn't needed because the `let deploy_id: i64 = ...` annotation on the left tells the compiler the target type.
- **`.map_err(|_| ...)`** — convert `ParseIntError` into our error. The closure ignores the parse error.
- **`&mut StderrShim`** — passes a no-op `Io` impl. The polling code in `check_deployment_progress` doesn't actually use `Io` (it only renders progress through the TUI), so we pass a dummy that ignores all messages.

### Helper functions

#### `fetch_deployments()`

```rust
pub fn fetch_deployments(http, stack, environment, start_date) -> Result<ApiResponse, Error> {
    let since_unix = strtotime_or_now(start_date);
    let query = format!("datestarted_from_unix={since_unix}");
    let relative_url = format!("project/{stack}/environment/{environment}/deploys?{query}");
    let response = http.request("GET", &relative_url, None, None)?;
    if response.status != 200 { return Err(Error::HttpStatus { /* ... */ }); }
    let body = decode_body(&response.body)?;
    Ok(ApiResponse { status: response.status, reason: response.reason, body })
}
```

Just a GET with a query parameter. Note the explicit check for status 200 — DeployNaut returns other codes if the date is in the future or the stack doesn't exist.

#### `check_deployment_progress()`

The polling core. Same pattern as `do_git_fetch` but with a longer deadline (`DEPLOY_TIMEOUT_SECS = 1800`).

#### `StderrShim`

```rust
struct StderrShim;
impl Io for StderrShim {
    fn warning(&mut self, _: &str) {}
    fn success(&mut self, _: &str) {}
    fn message(&mut self, _: &str) {}
    fn write_json(&mut self, _: &ApiResponse) {}
}
```

A no-op `Io` implementation. All methods take `_` (underscore) arguments — the underscore tells the compiler "I don't care about this argument." All bodies are empty (`{}`) — the methods do nothing.

This is needed because `check_deployment_progress` requires an `&mut dyn Io` parameter, but we don't have a real `Io` to pass at that point in the call stack. The shim satisfies the type system without doing anything.

#### `strtotime_or_now()`

```rust
fn strtotime_or_now(s: &str) -> i64 {
    if let Ok(n) = s.parse::<i64>() { return n; }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) { return dt.timestamp(); }
    match s.trim().to_ascii_lowercase().as_str() {
        "-1 year" | "last year" => (Utc::now() - chrono::Duration::days(365)).timestamp(),
        "-1 month" | "last month" => (Utc::now() - chrono::Duration::days(30)).timestamp(),
        "-1 day" | "yesterday" => (Utc::now() - chrono::Duration::days(1)).timestamp(),
        _ => Utc::now().timestamp(),
    }
}
```

A partial re-implementation of PHP's `strtotime`. Supports the formats used by the tests:

- Numeric Unix epoch (e.g., `"1700000000"`).
- RFC3339 timestamp (e.g., `"2026-12-25T10:00:00Z"`).
- Common relative strings: `"-1 year"`, `"last year"`, `"-1 month"`, etc.
- Anything else: fall back to "now."

**Teaching points:**

- **`chrono::Duration::days(365)`** — a `Duration` of 365 days. Subtracting it from `Utc::now()` gives us a moment in the past.
- **`Utc::now()`** — the current UTC time as a `DateTime<Utc>`.
- **`.timestamp()`** — extract the Unix epoch seconds as `i64`.

#### `decode_body()` and `poll_status()`

Tiny helpers. `decode_body` parses JSON or returns `Value::Null` on empty input. `poll_status` extracts the "status" field from the response and returns whether the operation is done.

---

## Why Is It Written This Way?

- **Every action is its own function.** Even though they share setup logic, each action is isolated. This makes them easy to test individually.
- **`DeploymentOverrides` is passed explicitly.** Rather than smuggling overrides through env vars or globals, the Bitbucket command builds an `Options` bag that already has the right values, then optionally passes overrides. This keeps the data flow explicit.
- **PHP parity.** Each action corresponds to a method on the PHP `DeployNautCommand` class. The HTTP shape, the JSON keys, the env vars — all preserved.
- **Composition over duplication.** `do_last_deployment` reuses `do_get_deployments`. `do_deploy_package` (in `bitbucket.rs`) reuses `do_create_deployment`. The codebase favors small, composable functions.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub struct Name;` | `DeployNaut` | Unit struct — just a tag for trait impl |
| `impl Command for DeployNaut` | top | Implement the `Command` trait |
| `match action.to_ascii_lowercase().as_str()` | dispatch | Match on a normalised string |
| `ok_or_else(\|_\| ...)` | option reads | Convert `Option` to `Result` with a lazy error |
| `?` | everywhere | Early-return on `Err` |
| `.ok()` | env vars | `Result<T, E>` → `Option<T>` |
| `.filter(\|v\| ...)` | env vars | Refine an `Option` |
| `Option<&T>::and_then` | JSON traversal | Chain nested lookups safely |
| `Vec::retain` | deployments filter | Keep only items matching a predicate |
| `Vec::sort_by` | deployments sort | Sort with a custom comparator |
| `unwrap_or_else(\|\| ...)` | default value | Default to a lazy computed value |
| `unwrap_or_default()` | missing data | Default to the type's `Default::default()` |
| `String::parse::<i64>()` | deploy_id | Parse a string into an integer |
| `.map_err(\|_\| ...)` | parse errors | Convert error types |
| `.cloned()` | cloned JSON | `Option<&T>` → `Option<T>` |
| `move` closure | polling closure | Move captured variables into the closure |
| `Duration::from_secs(n)` | timeouts | Create a duration from seconds |
| `format!("...{var}...")` | URLs | String interpolation |
| `json!({ "data": deployments })` | response body | Construct a JSON value |
| `#[allow(dead_code)]` | constants | Suppress unused-code warnings |
| `StderrShim` | private struct | No-op `Io` impl for internal use |
