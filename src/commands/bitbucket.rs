//! Bitbucket Pipelines actions. Mirrors the PHP `BitbucketCommand` class.
//!
//! - `createAccessToken` — POSTs to bitbucket.org/site/oauth2/access_token/
//!   with `grant_type=client_credentials`. Endpoint is overridable via
//!   `BB_OAUTH_ENDPOINT` env var (default `https://bitbucket.org/site`)
//!   so tests can point at a wiremock server.
//! - `createTag` — POSTs a tag ref to a Bitbucket repo. PHP truncates the tag
//!   string to `(commit_position + min(7, commit.len()))` chars when the tag
//!   contains the commit string (the PHP variable `$commit` is the full SHA,
//!   so this yields `tag_prefix + first_7_chars_of_commit`).
//! - `deployPackage` — calls `createAccessToken` internally, composes a
//!   package download URL, then calls the shared `do_create_deployment`
//!   helper with `--ref_type=package --ref=<download-url>`. Per the user's
//!   decision (Decision D6), the response is a **single combined
//!   `ApiResponse`** rather than the PHP three-line shape.

use serde_json::{json, Value};

use crate::commands::deploy_naut::{do_create_deployment, DeploymentOverrides};
use crate::commands::Command;
use crate::error::{ApiResponse, Error};
use crate::http::HttpClient;
use crate::io::Io;
use crate::options::Options;

const BITBUCKET_OAUTH_DEFAULT: &str = "https://bitbucket.org/site";

pub struct Bitbucket;

impl Command for Bitbucket {
    fn name(&self) -> &'static str {
        "ci:bitbucket"
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
            "createaccesstoken" => do_create_access_token(&opts, http),
            "createtag" => do_create_tag(&opts, http),
            "deploypackage" => do_deploy_package(&opts, io, http),
            _ => Err(Error::MissingAction),
        }
    }
}

/// `ci:bitbucket createAccessToken`.
pub fn do_create_access_token(
    _opts: &Options,
    http: &mut dyn HttpClient,
) -> Result<ApiResponse, Error> {
    let consumer_key = std::env::var("BB_CONSUMER_KEY")
        .map_err(|_| Error::MissingEnv("BB_CONSUMER_KEY".into()))?;
    let consumer_secret = std::env::var("BB_CONSUMER_SECRET")
        .map_err(|_| Error::MissingEnv("BB_CONSUMER_SECRET".into()))?;

    let endpoint = std::env::var("BB_OAUTH_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| BITBUCKET_OAUTH_DEFAULT.to_string());
    http.set_endpoint(&endpoint);
    http.set_content_type("application/x-www-form-urlencoded");
    http.set_username_and_password(&consumer_key, &consumer_secret);

    let resp = http.request(
        "POST",
        "oauth2/access_token/",
        None,
        Some("grant_type=client_credentials".into()),
    )?;
    if resp.status != 200 {
        return Err(Error::HttpStatus {
            status: resp.status,
            reason: resp.reason,
            body: resp.body,
        });
    }
    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse {
        status: resp.status,
        reason: resp.reason,
        body,
    })
}

/// `ci:bitbucket createTag`.
pub fn do_create_tag(opts: &Options, http: &mut dyn HttpClient) -> Result<ApiResponse, Error> {
    let commit = opts
        .option("commit")
        .ok_or_else(|| Error::MissingOption("commit".into()))?;
    if commit.len() != 40 {
        return Err(Error::Generic(
            "[Action:DeployPackage] Requires 40-char commit".into(),
        ));
    }
    let mut tag = opts
        .option("tag")
        .ok_or_else(|| Error::MissingOption("tag".into()))?;

    let (owner, slug) = match (
        std::env::var("BITBUCKET_REPO_OWNER").ok(),
        std::env::var("BITBUCKET_REPO_SLUG").ok(),
    ) {
        (Some(o), Some(s)) if !o.is_empty() && !s.is_empty() => (o, s),
        _ => {
            return Err(Error::MissingEnv(
                "BITBUCKET_REPO_OWNER/BITBUCKET_REPO_SLUG".into(),
            ))
        }
    };

    let endpoint = std::env::var("BB_OAUTH_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .map(|_| std::env::var("BB_ENDPOINT").unwrap_or_default())
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var("BB_ENDPOINT").ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "https://api.bitbucket.org/2.0".to_string());
    http.set_endpoint(&endpoint);

    // PHP `strrpos($tag, $commit)` — substring truncation: if the tag
    // string contains the full commit, keep up to 7 chars of the commit.
    if let Some(pos) = tag.rfind(&commit) {
        let cap = std::cmp::min(7, commit.len());
        tag.truncate(pos + cap);
    }

    let url = format!("repositories/{owner}/{slug}/refs/tags");
    let payload = json!({
        "name": tag,
        "target": { "hash": commit },
    });
    let body_str =
        serde_json::to_string(&payload).map_err(|e| Error::Json(format!("encode failed: {e}")))?;
    let resp = http.request("POST", &url, None, Some(body_str))?;
    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| Error::Json(format!("decode failed: {e}")))?;
    Ok(ApiResponse {
        status: resp.status,
        reason: resp.reason,
        body,
    })
}

/// `ci:bitbucket deployPackage`. Per Decision D6, returns a single combined
/// `ApiResponse` (the create-deployment response) rather than the PHP
/// three-line shape.
pub fn do_deploy_package(
    opts: &Options,
    _io: &mut dyn Io,
    http: &mut dyn HttpClient,
) -> Result<ApiResponse, Error> {
    let commit = opts
        .option("commit")
        .ok_or_else(|| Error::MissingOption("commit".into()))?;
    if commit.len() != 40 {
        return Err(Error::Generic(
            "[Action:DeployPackage] Requires 40-char commit".into(),
        ));
    }
    // Validate stack and environment are present (do_create_deployment
    // would also error, but failing fast gives a clearer message).
    let _stack = opts
        .option("stack")
        .ok_or_else(|| Error::MissingOption("stack".into()))?;
    let _environment = opts
        .option("environment")
        .ok_or_else(|| Error::MissingOption("environment".into()))?;
    let branch = std::env::var("BITBUCKET_BRANCH").ok();
    let owner = std::env::var("BITBUCKET_REPO_OWNER")
        .map_err(|_| Error::MissingEnv("BITBUCKET_REPO_OWNER".into()))?;
    let slug = std::env::var("BITBUCKET_REPO_SLUG")
        .map_err(|_| Error::MissingEnv("BITBUCKET_REPO_SLUG".into()))?;
    let endpoint = std::env::var("BB_ENDPOINT").map_err(|_| Error::MissingEndpoint)?;

    // Step 1: create the access token (this mutates the http client's
    // endpoint / content-type / authorization).
    let token_response = do_create_access_token(opts, http)?;
    eprintln!("[bitbucket] token_response: {:?}", token_response.body);
    let access_token = token_response
        .body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::Generic("[Action:DeployPackage] access_token missing from response".into())
        })?
        .to_string();

    // Step 2: build the download URL.
    let download_link = format!(
        "{}/repositories/{}/{}/downloads/{}.tar.gz?access_token={}",
        endpoint.trim_end_matches('/'),
        owner,
        slug,
        commit,
        access_token
    );

    // Step 3: create the deployment with package overrides.
    let overrides = DeploymentOverrides {
        ref_: Some(download_link),
        ref_type: Some("package".into()),
        title: Some(format!("[CD:Package] {commit}")),
        summary: Some(format!("Branch:{}", branch.as_deref().unwrap_or(""))),
    };

    // The PHP version passes bypass_and_start / should_wait through to
    // create_deployment. They're already on the options bag.
    do_create_deployment(opts, _io, http, Some(overrides))
}
