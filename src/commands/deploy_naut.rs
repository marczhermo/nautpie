//! DeployNaut API actions. Mirrors the PHP `DeployNautCommand` class —
//! one method per action, all reachable via the `deploy:naut <action>`
//! subcommand. The HTTP layer is injected so tests can substitute a
//! `wiremock`-backed `ReqwestHttpClient` (or any other `HttpClient` impl).

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::boolean::check_boolean;
use crate::commands::sample;
use crate::commands::Command;
use crate::deployment_details::DeploymentDetails;
use crate::env::{check_envs, is_production};
use crate::error::{ApiResponse, Error};
use crate::http::HttpClient;
use crate::io::Io;
use crate::options::Options;
use crate::tui::{run_with_progress, PollOutcome};

/// DeployNaut deploy polling timeout in seconds (matches PHP const).
pub const DEPLOY_TIMEOUT_SECS: u64 = 1800;
/// Git-fetch polling timeout in seconds (matches PHP const).
pub const GIT_TIMEOUT_SECS: u64 = 120;

pub struct DeployNaut;

impl Command for DeployNaut {
    fn name(&self) -> &'static str {
        "deploy:naut"
    }

    fn run(
        &self,
        action: &str,
        io: &mut dyn Io,
        http: &mut dyn HttpClient,
    ) -> Result<ApiResponse, Error> {
        let opts = io
            .options()
            .ok_or_else(|| Error::MissingOption("options".into()))?;
        match action.to_ascii_lowercase().as_str() {
            "samplesuccess" => Ok(sample::sample_success()),
            "samplefail" => sample::sample_fail(),
            "fetch" => do_fetch(&opts, http),
            "createdeployment" => do_create_deployment(&opts, io, http, None),
            "gitfetch" => do_git_fetch(&opts, http),
            "getdeployments" => do_get_deployments(&opts, http),
            "lastdeployment" => do_last_deployment(&opts, http),
            "checkdeploymentprogress" => do_check_deployment_progress(&opts, http),
            _ => Err(Error::MissingAction),
        }
    }
}

/// Build an Options from the parsed clap DeployNautArgs.
pub fn options_from_deploy_args(args: &crate::cli::DeployNautArgs) -> Options {
    let mut opts = Options::new();
    if let Some(v) = &args.url {
        opts.set("url", v.clone());
    }
    if let Some(v) = &args.commit {
        opts.set("commit", v.clone());
    }
    if let Some(v) = &args.stack {
        opts.set("stack", v.clone());
    }
    if let Some(v) = &args.environment {
        opts.set("environment", v.clone());
    }
    if let Some(v) = &args.start_date {
        opts.set("startDate", v.clone());
    }
    if let Some(v) = &args.title {
        opts.set("title", v.clone());
    }
    if let Some(v) = &args.summary {
        opts.set("summary", v.clone());
    }
    if let Some(v) = &args.redeploy {
        opts.set("redeploy", v.clone());
    }
    if let Some(v) = &args.r#ref {
        opts.set("ref", v.clone());
    }
    if let Some(v) = &args.ref_type {
        opts.set("ref_type", v.clone());
    }
    if let Some(v) = &args.bypass_and_start {
        opts.set("bypass_and_start", v.clone());
    }
    if let Some(v) = &args.deploy_id {
        opts.set("deploy_id", v.clone());
    }
    if let Some(v) = &args.should_wait {
        opts.set("should_wait", v.clone());
    }
    opts
}

/// Build an Options from the parsed clap BitbucketArgs.
pub fn options_from_bitbucket_args(args: &crate::cli::BitbucketArgs) -> Options {
    let mut opts = Options::new();
    opts.set("action", args.action.clone());
    if let Some(v) = &args.commit {
        opts.set("commit", v.clone());
    }
    if let Some(v) = &args.stack {
        opts.set("stack", v.clone());
    }
    if let Some(v) = &args.environment {
        opts.set("environment", v.clone());
    }
    if let Some(v) = &args.title {
        opts.set("title", v.clone());
    }
    if let Some(v) = &args.summary {
        opts.set("summary", v.clone());
    }
    if let Some(v) = &args.tag {
        opts.set("tag", v.clone());
    }
    if let Some(v) = &args.bypass_and_start {
        opts.set("bypass_and_start", v.clone());
    }
    if let Some(v) = &args.deploy_id {
        opts.set("deploy_id", v.clone());
    }
    if let Some(v) = &args.should_wait {
        opts.set("should_wait", v.clone());
    }
    opts
}

/// Apply environment defaults from `NAUT_ENDPOINT`, `DASH_USER`, `DASH_TOKEN`.
pub fn setup_deploy_naut_env(http: &mut dyn HttpClient) -> Result<(String, String, String), Error> {
    let endpoint = std::env::var("NAUT_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(Error::MissingEndpoint)?;
    http.set_endpoint(&endpoint);
    // Reset content-type: do_deploy_package sets it to form-urlencoded for
    // the OAuth token call and then immediately creates a deployment. We
    // need JSON content-type for the deployment POST.
    http.set_content_type("application/json");
    let creds = check_envs(&["DASH_USER", "DASH_TOKEN"])?;
    let dash_user = creds[0].clone();
    let dash_token = creds[1].clone();
    http.set_username_and_password(&dash_user, &dash_token);
    Ok((endpoint, dash_user, dash_token))
}

// ---- Actions ----------------------------------------------------------------

/// `deploy:naut fetch` — GETs `{endpoint}/{url}` and returns the JSON body.
pub fn do_fetch(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let url = opts
        .option("url")
        .ok_or_else(|| Error::MissingOption("url".into()))?;
    let resp = http.request("GET", &url, None, None)?;
    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse {
        status: resp.status,
        reason: resp.reason,
        body,
    })
}

/// Optional overrides for `create_deployment_with` — used by `do_deploy_package`.
#[derive(Default, Clone)]
pub struct DeploymentOverrides {
    pub ref_: Option<String>,
    pub ref_type: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

/// `deploy:naut createDeployment` — POST to DeployNaut.
pub fn do_create_deployment(
    opts: &Options,
    io: &mut dyn Io,
    http: &mut dyn HttpClient,
    overrides: Option<DeploymentOverrides>,
) -> Result<ApiResponse, Error> {
    setup_deploy_naut_env(http)?;

    let stack = opts
        .option("stack")
        .ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts
        .option("environment")
        .ok_or_else(|| Error::MissingOption("environment".into()))?;
    let ref_type = overrides
        .as_ref()
        .and_then(|o| o.ref_type.clone())
        .or_else(|| opts.option("ref_type"))
        .ok_or_else(|| Error::MissingOption("ref_type".into()))?;

    let mut details = DeploymentDetails::new().ref_type(ref_type.clone());
    if let Some(r) = overrides.as_ref().and_then(|o| o.ref_.clone()) {
        details = details.ref_(r);
    } else if let Some(r) = opts.option("ref") {
        details = details.ref_(r);
    }
    if let Some(t) = overrides.as_ref().and_then(|o| o.title.clone()) {
        details = details.title(t);
    } else if let Some(t) = opts.option("title") {
        details = details.title(t);
    }
    if let Some(s) = overrides.as_ref().and_then(|o| o.summary.clone()) {
        details = details.summary(s);
    } else if let Some(s) = opts.option("summary") {
        details = details.summary(s);
    }

    if let Some(start) = opts.option("startDate") {
        details = details.schedule_to_start(start);
    }

    let bypass_and_start = check_boolean(opts.option("bypass_and_start").as_deref())?;
    if bypass_and_start && !is_production(&environment) {
        details = details.bypass_and_start(true);
    }

    let redeploy = check_boolean(opts.option("redeploy").as_deref())?;
    if redeploy && !is_production(&environment) {
        details = details.redeploy(true);
    }

    let has_ref =
        overrides.as_ref().and_then(|o| o.ref_.clone()).is_some() || opts.option("ref").is_some();
    if !has_ref && !redeploy && ref_type != "promote_from_uat" {
        return Err(Error::Generic(
            "[Action:CreateDeployment] Requires ref option".into(),
        ));
    }

    let relative_url = format!("project/{stack}/environment/{environment}/deploys");
    let body_str = serde_json::to_string(&details.values())
        .map_err(|e| Error::Json(format!("encode failed: {e}")))?;
    let response = http.request("POST", &relative_url, None, Some(body_str))?;

    let should_wait = check_boolean(opts.option("should_wait").as_deref())?;
    let body = decode_body(&response.body)?;
    let deploy_id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|i| i.as_i64())
        .or_else(|| {
            body.get("data")
                .and_then(|d| d.get("id"))
                .and_then(|i| i.as_str())
                .and_then(|s| s.parse::<i64>().ok())
        });

    if should_wait {
        if let Some(id) = deploy_id {
            check_deployment_progress(id, &stack, &environment, io, http)?;
        }
    }

    Ok(ApiResponse {
        status: response.status,
        reason: response.reason,
        body,
    })
}

/// `deploy:naut gitFetch` — POST to git/fetches and poll until Complete.
pub fn do_git_fetch(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let stack = opts
        .option("stack")
        .ok_or_else(|| Error::MissingOption("stack".into()))?;

    setup_deploy_naut_env(http)?;
    let relative_url = format!("project/{stack}/git/fetches");
    let response = http.request("POST", &relative_url, None, None)?;

    if response.status != 202 {
        return Err(Error::HttpStatus {
            status: response.status,
            reason: response.reason,
            body: response.body.clone(),
        });
    }

    let body = decode_body(&response.body)?;
    let fetch_id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|i| i.as_i64())
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
    let final_resp = http.request("GET", &poll_url, None, None)?;
    let final_body = decode_body(&final_resp.body)?;
    Ok(ApiResponse {
        status: final_resp.status,
        reason: final_resp.reason,
        body: final_body,
    })
}

/// `deploy:naut getDeployments`.
pub fn do_get_deployments(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let stack = opts
        .option("stack")
        .ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts
        .option("environment")
        .ok_or_else(|| Error::MissingOption("environment".into()))?;
    let start_date = opts.option("startDate").unwrap_or_else(|| "-1 year".into());

    setup_deploy_naut_env(http)?;
    let response = fetch_deployments(http, &stack, &environment, &start_date)?;

    let mut deployments = response
        .body
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    if let Some(commit) = opts.option("commit") {
        let commit_key = if commit.len() == 7 {
            "short_sha"
        } else {
            "sha"
        };
        let accepted_states = [
            "New",
            "Submitted",
            "Approved",
            "Queued",
            "Deploying",
            "Completed",
        ];
        deployments.retain(|item| {
            item.get("attributes")
                .and_then(|a| a.get(commit_key))
                .and_then(|v| v.as_str())
                == Some(commit.as_str())
                && item
                    .get("attributes")
                    .and_then(|a| a.get("state"))
                    .and_then(|v| v.as_str())
                    .map(|s| accepted_states.contains(&s))
                    .unwrap_or(false)
        });
    }

    deployments.sort_by(|a, b| {
        let aid = a.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
        let bid = b.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
        bid.cmp(&aid)
    });

    Ok(ApiResponse {
        status: response.status,
        reason: response.reason,
        body: json!({ "data": deployments }),
    })
}

/// `deploy:naut lastDeployment`.
pub fn do_last_deployment(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let mut response = do_get_deployments(opts, http)?;
    let first = response
        .body
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first().cloned())
        .unwrap_or(Value::Null);
    response.body = first;
    Ok(response)
}

/// `deploy:naut checkDeploymentProgress`.
pub fn do_check_deployment_progress(
    opts: &Options,
    http: &mut dyn HttpClient,
) -> Result<ApiResponse, Error> {
    let stack = opts
        .option("stack")
        .ok_or_else(|| Error::MissingOption("stack".into()))?;
    let environment = opts
        .option("environment")
        .ok_or_else(|| Error::MissingOption("environment".into()))?;
    let deploy_id_str = opts
        .option("deploy_id")
        .ok_or_else(|| Error::MissingOption("deploy_id".into()))?;
    let deploy_id: i64 = deploy_id_str
        .parse()
        .map_err(|_| Error::Generic("[Required:Option] deploy_id must be an integer".into()))?;
    check_deployment_progress(deploy_id, &stack, &environment, &mut StderrShim, http)?;
    Ok(ApiResponse::ok(json!("Deployment progress check complete")))
}

/// Low-level: GET deployments for a stack/env from a start date.
pub fn fetch_deployments(
    http: &mut dyn HttpClient,
    stack: &str,
    environment: &str,
    start_date: &str,
) -> Result<ApiResponse, Error> {
    let since_unix = strtotime_or_now(start_date);
    let query = format!("datestarted_from_unix={since_unix}");
    let relative_url = format!("project/{stack}/environment/{environment}/deploys?{query}");
    let response = http.request("GET", &relative_url, None, None)?;
    if response.status != 200 {
        return Err(Error::HttpStatus {
            status: response.status,
            reason: response.reason,
            body: response.body.clone(),
        });
    }
    let body = decode_body(&response.body)?;
    Ok(ApiResponse {
        status: response.status,
        reason: response.reason,
        body,
    })
}

/// Poll a deployment's progress until it reaches the `Completed` state.
pub fn check_deployment_progress(
    deploy_id: i64,
    stack: &str,
    environment: &str,
    _io: &mut dyn Io,
    http: &mut dyn HttpClient,
) -> Result<(), Error> {
    let poll_url = format!("project/{stack}/environment/{environment}/deploys/{deploy_id}");
    let label = format!("Deployment #{deploy_id} for {stack}/{environment}");
    run_with_progress(
        "checkDeploymentProgress",
        label,
        Duration::from_secs(DEPLOY_TIMEOUT_SECS),
        http,
        move |http| poll_status(http, &poll_url, "state"),
    )
}

/// A no-op Io impl used internally so we don't need to plumb the user's
/// Io down into helpers. The check_deployment_progress helper only emits
/// progress through the TUI; the Io sink isn't required.
struct StderrShim;
impl Io for StderrShim {
    fn warning(&mut self, _: &str) {}
    fn success(&mut self, _: &str) {}
    fn message(&mut self, _: &str) {}
    fn write_json(&mut self, _: &ApiResponse) {}
}

// ---- Helpers ----------------------------------------------------------------

/// PHP `strtotime` parity for the subset used by the tests:
/// numeric Unix epoch, RFC3339, `-1 year`, `-1 month`, `-1 day`,
/// `yesterday`, `last year`, `last month`. Anything else falls back to
/// the current epoch.
fn strtotime_or_now(s: &str) -> i64 {
    if let Ok(n) = s.parse::<i64>() {
        return n;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.timestamp();
    }
    match s.trim().to_ascii_lowercase().as_str() {
        "-1 year" | "last year" => (Utc::now() - chrono::Duration::days(365)).timestamp(),
        "-1 month" | "last month" => (Utc::now() - chrono::Duration::days(30)).timestamp(),
        "-1 day" | "yesterday" => (Utc::now() - chrono::Duration::days(1)).timestamp(),
        _ => Utc::now().timestamp(),
    }
}

fn decode_body(raw: &str) -> Result<Value, Error> {
    if raw.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(raw).map_err(|e| Error::Json(format!("decode failed: {e}")))
}

fn poll_status(
    http: &mut dyn HttpClient,
    url: &str,
    status_key: &str,
) -> Result<PollOutcome, Error> {
    let resp = http.request("GET", url, None, None)?;
    let body = decode_body(&resp.body)?;
    let status = body
        .get("data")
        .and_then(|d| d.get("attributes"))
        .and_then(|a| a.get(status_key))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let complete_markers = ["Complete", "Completed"];
    if complete_markers.contains(&status.as_str()) {
        Ok(PollOutcome::Done)
    } else {
        Ok(PollOutcome::Pending(if status.is_empty() {
            "in progress".into()
        } else {
            status
        }))
    }
}
