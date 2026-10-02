# `src/commands/bitbucket.rs` — The Bitbucket Pipelines Actions

This file implements the `ci:bitbucket <action>` subcommand, which integrates with Bitbucket Pipelines. It mirrors the PHP `BitbucketCommand` class.

While `deploy_naut.rs` is large and self-contained, this file is **smaller** but **more dependent** — `do_deploy_package` calls back into `deploy_naut.rs` to do the actual deployment after getting a Bitbucket download URL.

---

## The Big Picture

`ci:bitbucket` supports three actions:

| Action | What it does |
|---|---|
| `createAccessToken` | POSTs to Bitbucket's OAuth endpoint with `grant_type=client_credentials` to get a temporary access token. |
| `createTag` | POSTs a tag reference to a Bitbucket repo. The tag string is **truncated** to match a PHP quirk. |
| `deployPackage` | Combines `createAccessToken` + building a download URL + calling `do_create_deployment`. |

The `do_deploy_package` action is the one that ties Bitbucket and DeployNaut together: it gets a token, builds a download URL, and then **calls into the `deploy_naut` module** to perform the actual deployment.

---

## Walkthrough

### Constant

```rust
const BITBUCKET_OAUTH_DEFAULT: &str = "https://bitbucket.org/site";
```

The default Bitbucket OAuth endpoint. Used unless `BB_OAUTH_ENDPOINT` is set in the environment (which lets tests point at a `wiremock` server).

### The `Bitbucket` struct and `Command` impl

```rust
pub struct Bitbucket;

impl Command for Bitbucket {
    fn name(&self) -> &'static str { "ci:bitbucket" }

    fn run(&self, action: &str, io: &mut dyn Io, http: &mut dyn HttpClient)
        -> Result<ApiResponse, Error>
    {
        let opts = io.options().ok_or_else(|| Error::MissingOption("options".into()))?;
        match action.to_ascii_lowercase().as_str() {
            "createaccesstoken" => do_create_access_token(&opts, http),
            "createtag"         => do_create_tag(&opts, http),
            "deploypackage"     => do_deploy_package(&opts, io, http),
            _ => Err(Error::MissingAction),
        }
    }
}
```

Same structure as `DeployNaut::run()`. The match arms route to the three action functions.

### `do_create_access_token()`

```rust
pub fn do_create_access_token(_opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let consumer_key = std::env::var("BB_CONSUMER_KEY").map_err(|_| Error::MissingEnv("BB_CONSUMER_KEY".into()))?;
    let consumer_secret = std::env::var("BB_CONSUMER_SECRET").map_err(|_| Error::MissingEnv("BB_CONSUMER_SECRET".into()))?;

    let endpoint = std::env::var("BB_OAUTH_ENDPOINT").ok().filter(|v| !v.is_empty())
        .unwrap_or_else(|| BITBUCKET_OAUTH_DEFAULT.to_string());
    http.set_endpoint(&endpoint);
    http.set_content_type("application/x-www-form-urlencoded");
    http.set_username_and_password(&consumer_key, &consumer_secret);

    let resp = http.request("POST", "oauth2/access_token/", None, Some("grant_type=client_credentials".into()))?;
    if resp.status != 200 {
        return Err(Error::HttpStatus { status: resp.status, reason: resp.reason, body: resp.body });
    }
    let body: Value = serde_json::from_str(&resp.body).map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse { status: resp.status, reason: resp.reason, body })
}
```

This is the OAuth client-credentials flow. Bitbucket returns:

```json
{ "access_token": "...", "scopes": "...", "expires_in": 3600 }
```

**Teaching points:**

- **`_opts: &Options`** — the underscore prefix means "I don't use this parameter." The `_opts` is there because the trait signature requires it.
- **`std::env::var(...).map_err(|_| Error::MissingEnv("NAME".into()))?`** — the standard pattern for "env var or error." `map_err` converts the standard library's `VarError` into our `Error::MissingEnv`. The `?` propagates the error.
- **`.ok().filter(|v| !v.is_empty()).unwrap_or_else(...)`** — three operations chained:
  - `.ok()` — convert `Result` to `Option`.
  - `.filter(...)` — keep the value only if it's non-empty (so an unset-but-empty var doesn't override the default).
  - `.unwrap_or_else(...)` — fall back to the default.
- **`http.set_content_type("application/x-www-form-urlencoded")`** — the access token request is a form submission, not JSON.
- **`http.set_username_and_password(&consumer_key, &consumer_secret)`** — sets the HTTP basic-auth header. Bitbucket's OAuth expects the consumer key and secret as basic-auth credentials.
- **`Some("grant_type=client_credentials".into())`** — passes the form-encoded body. The `.into()` converts the `&str` to a `String` so it matches the `Option<String>` parameter type.
- **The trailing `if resp.status != 200`** — explicitly check for success. Other status codes (401, 403, etc.) become `Error::HttpStatus`.

### `do_create_tag()` — the truncation quirk

```rust
pub fn do_create_tag(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let commit = opts.option("commit").ok_or_else(|| Error::MissingOption("commit".into()))?;
    if commit.len() != 40 {
        return Err(Error::Generic("[Action:DeployPackage] Requires 40-char commit".into()));
    }
    let mut tag = opts.option("tag").ok_or_else(|| Error::MissingOption("tag".into()))?;

    let (owner, slug) = match (std::env::var("BITBUCKET_REPO_OWNER").ok(), std::env::var("BITBUCKET_REPO_SLUG").ok()) {
        (Some(o), Some(s)) if !o.is_empty() && !s.is_empty() => (o, s),
        _ => return Err(Error::MissingEnv("BITBUCKET_REPO_OWNER/BITBUCKET_REPO_SLUG".into())),
    };

    let endpoint = /* ... */;
    http.set_endpoint(&endpoint);

    // PHP `strrpos($tag, $commit)` — substring truncation:
    // if the tag string contains the full commit, keep up to 7 chars of the commit.
    if let Some(pos) = tag.rfind(&commit) {
        let cap = std::cmp::min(7, commit.len());
        tag.truncate(pos + cap);
    }

    let url = format!("repositories/{owner}/{slug}/refs/tags");
    let payload = json!({
        "name": tag,
        "target": { "hash": commit },
    });
    let body_str = serde_json::to_string(&payload).map_err(|e| Error::Json(format!("encode failed: {e}")))?;
    let resp = http.request("POST", &url, None, Some(body_str))?;
    let body: Value = serde_json::from_str(&resp.body).map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse { status: resp.status, reason: resp.reason, body })
}
```

This is the most idiosyncratic function in the file because of the **PHP truncation quirk**.

**Teaching points:**

- **`if commit.len() != 40`** — Bitbucket SHA-1s are exactly 40 hex characters. Anything else is rejected.
- **`let mut tag = ...`** — `mut` because we'll truncate it later. Without `mut`, you can't reassign or call mutating methods on a variable.
- **`match (a, b)`** — Rust's tuple pattern matching. The first arm matches when both env vars are `Some(non-empty)`; the second arm catches everything else (including both `None`, one `None` one `Some`, or both `Some` but one empty). The `if !o.is_empty() && !s.is_empty()` is a **match guard** that refines the first arm.
- **`tag.rfind(&commit)`** — find the last occurrence of `commit` in `tag`. Returns `Option<usize>` (the byte position, or `None`).
- **`tag.truncate(pos + cap)`** — keep the first `pos + cap` bytes of the tag, drop the rest. This is the **PHP `substr($tag, 0, strrpos($tag, $commit) + min(7, strlen($commit)))`** behavior.
- **`std::cmp::min(7, commit.len())`** — `commit.len()` is 40, so `min(7, 40) == 7`. The `min` is defensive: if a future version supports shorter commits, this still works.
- **`json!({ "name": tag, "target": { "hash": commit } })`** — the `json!` macro supports nested object syntax. It produces `{"name": "...", "target": {"hash": "..."}}`.

### Why the truncation?

The PHP original had this line:

```php
$tag = substr($tag, 0, strrpos($tag, $commit) + min(7, strlen($commit)));
```

The intent (according to the comments in the source) was: "if the tag string contains the full 40-char commit, truncate to the first 7 chars of the commit so the tag is shorter." This was probably for some legacy reason — modern tags don't need this. The Rust port preserves the exact behavior for PHP parity.

### `do_deploy_package()` — composition

```rust
pub fn do_deploy_package(opts: &Options, _io: &mut dyn Io, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let commit = opts.option("commit").ok_or_else(|| Error::MissingOption("commit".into()))?;
    if commit.len() != 40 { return Err(Error::Generic("...".into())); }

    let _stack = opts.option("stack").ok_or_else(|| Error::MissingOption("stack".into()))?;
    let _environment = opts.option("environment").ok_or_else(|| Error::MissingOption("environment".into()))?;

    let branch = std::env::var("BITBUCKET_BRANCH").ok();
    let owner = std::env::var("BITBUCKET_REPO_OWNER").map_err(|_| Error::MissingEnv("BITBUCKET_REPO_OWNER".into()))?;
    let slug = std::env::var("BITBUCKET_REPO_SLUG").map_err(|_| Error::MissingEnv("BITBUCKET_REPO_SLUG".into()))?;
    let endpoint = std::env::var("BB_ENDPOINT").map_err(|_| Error::MissingEndpoint)?;

    let token_response = do_create_access_token(opts, http)?;
    eprintln!("[bitbucket] token_response: {:?}", token_response.body);
    let access_token = token_response.body.get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Generic("[Action:DeployPackage] access_token missing from response".into()))?
        .to_string();

    let download_link = format!(
        "{}/repositories/{}/{}/downloads/{}.tar.gz?access_token={}",
        endpoint.trim_end_matches('/'),
        owner, slug, commit, access_token
    );

    let overrides = DeploymentOverrides {
        ref_: Some(download_link),
        ref_type: Some("package".into()),
        title: Some(format!("[CD:Package] {commit}")),
        summary: Some(format!("Branch:{}", branch.as_deref().unwrap_or(""))),
    };

    do_create_deployment(opts, _io, http, Some(overrides))
}
```

This is the **most cross-cutting action** in the codebase. It:

1. Validates that all required env vars and CLI options are present.
2. Calls `do_create_access_token` to get a temporary token (this mutates the `http` client's endpoint, content type, and basic auth).
3. Builds a download URL of the form `https://api.bitbucket.org/2.0/repositories/{owner}/{slug}/downloads/{commit}.tar.gz?access_token={token}`.
4. Constructs a `DeploymentOverrides` that points `ref` at the download URL and sets `ref_type` to `"package"`.
5. Calls `do_create_deployment` (from `deploy_naut.rs`) with those overrides.

**Teaching points:**

- **`let _stack = ...`** — the leading underscore tells the compiler "I know this is unused; I'm just doing this for the side effect of validating that it exists." Without the underscore, the compiler would warn that the variable is never used.
- **`do_create_access_token(opts, http)?`** — calls the previous action function **directly**, not through the dispatcher. This is a deliberate shortcut: the dispatcher is for CLI-level routing; within the implementation, we just call functions.
- **`eprintln!(...)`** — debug output on stderr. This is here to help diagnose OAuth issues during development. In a more polished codebase, you'd probably remove it or gate it behind a debug flag.
- **`endpoint.trim_end_matches('/')`** — strip a trailing slash from the endpoint before appending the URL. This prevents double-slashes.
- **`DeploymentOverrides { ref_: ..., ref_type: ..., title: ..., summary: ... }`** — struct literal with the `Some(...)` wrappers. Note the field is `ref_` (with trailing underscore) because `ref` is a Rust keyword.
- **`format!("[CD:Package] {commit}")`** — `[CD:Package]` is the dashboard title prefix; `{commit}` interpolates the commit SHA. The result looks like `[CD:Package] abc123def456...`.
- **`do_create_deployment(opts, _io, http, Some(overrides))`** — calls into the other command module. The `Some(overrides)` tells it to use our synthesised ref/title/summary instead of whatever the user supplied on the CLI.

### The PHP three-line-vs-one-response divergence

The original PHP `do_deploy_package` printed **three** separate JSON lines:

1. The token response.
2. The deployment response.
3. The final combined response.

The Rust port, by decision D6, prints **only one** — the final combined response. This was a deliberate design choice: it's cleaner and the downstream CI scripts can be updated to expect a single line. The comment in the source notes this.

---

## Why Is It Written This Way?

- **PHP parity, with one fix.** The PHP version's tag-truncation logic was probably a bug, but it's preserved exactly because changing it would alter observable behavior. The port is meant to be **byte-compatible** with the PHP version.
- **Composition over duplication.** `do_deploy_package` reuses `do_create_access_token` and `do_create_deployment`. It doesn't reinvent either.
- **Single response.** Despite the PHP version producing three lines, the Rust port produces one. This was an intentional design choice to make CI scripts easier to write.
- **Defensive validation.** The function checks `commit.len() == 40` even though `do_create_deployment` would also reject it. Failing fast gives a clearer error message.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub struct Name;` | `Bitbucket` | Unit struct |
| `impl Command for Bitbucket` | top | Implement the `Command` trait |
| `match (a, b)` | `do_create_tag` | Match on a tuple |
| Match guard `if !o.is_empty() ...` | `do_create_tag` | Refine a pattern |
| `mut tag` | `do_create_tag` | Mutable variable for in-place truncation |
| `String::rfind(&commit)` | `do_create_tag` | Find last occurrence of substring |
| `String::truncate(n)` | `do_create_tag` | Truncate string to first `n` bytes |
| `std::cmp::min(a, b)` | `do_create_tag` | Smaller of two integers |
| `eprintln!(...)` | `do_deploy_package` | Print to stderr (debug) |
| `endpoint.trim_end_matches('/')` | `do_deploy_package` | Strip trailing characters |
| `_stack`, `_environment` | `do_deploy_package` | Discarded bindings (side-effect only) |
| `DeploymentOverrides` struct | `do_deploy_package` | Override fields in the deployment payload |
| `Some(field)` in struct literal | `do_deploy_package` | Wrap fields in `Option::Some` |
| Cross-module function call | `do_deploy_package` | Reuses `do_create_deployment` |
| `.to_string()` | `do_deploy_package` | Convert `&str` to owned `String` |
| `#[allow(dead_code)]` | trait | Suppress unused-code warnings |
