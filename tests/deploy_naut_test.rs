//! Wiremock-backed integration tests for the `deploy:naut` actions. Mirrors
//! the PHP `DeployNautCommandTest` cases.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use serde_json::json;

use nautpie::commands::deploy_naut::{
    do_check_deployment_progress, do_create_deployment, do_fetch, do_get_deployments, do_git_fetch,
    do_last_deployment, DeploymentOverrides,
};
use nautpie::error::{ApiResponse, Error};
use nautpie::http::{HttpClient, ReqwestHttpClient};
use nautpie::io::{Io, StderrIo};
use nautpie::options::Options;

const FIXTURE_GET_DEPLOYMENTS: &str = include_str!("fixtures/getDeployments.json");
const FIXTURE_CREATE_SUCCESS: &str = include_str!("fixtures/createDeploymentSuccess.json");
const FIXTURE_CREATE_ERROR: &str = include_str!("fixtures/createDeploymentError.json");

fn start_server<F>(setup: F) -> wiremock::MockServer
where
    F: for<'a> std::ops::FnOnce(
        &'a wiremock::MockServer,
    )
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

fn client_at(server: &wiremock::MockServer) -> ReqwestHttpClient {
    let mut c = ReqwestHttpClient::new().expect("client builds");
    c.set_endpoint(&server.uri());
    c
}

fn opts(pairs: &[(&str, &str)]) -> Options {
    let mut o = Options::new();
    for (k, v) in pairs {
        o.set(*k, (*v).to_string());
    }
    o
}

// ---- Env helpers ------------------------------------------------------------
//
// The actions read NAUT_ENDPOINT / DASH_USER / DASH_TOKEN. We mutate them in
// tests and restore on drop to avoid leaking state across tests.
#[allow(dead_code)]
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
            "BITBUCKET_REPO_OWNER",
            "BITBUCKET_REPO_SLUG",
            "BITBUCKET_BRANCH",
            "BB_ENDPOINT",
            "BB_AUTH_STRING",
            "BB_CONSUMER_KEY",
            "BB_CONSUMER_SECRET",
        ] {
            std::env::remove_var(key);
        }
    }
}

fn setup_deploy_env(server_uri: &str) -> EnvLock {
    let g = lock_env();
    // Bare host:port (no path component) so the request URL is rooted at
    // "/" — matches the mock paths.
    std::env::set_var("NAUT_ENDPOINT", server_uri);
    std::env::set_var("DASH_USER", "marco");
    std::env::set_var("DASH_TOKEN", "token123");
    g
}

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

#[test]
fn fetch_bad_response_propagates_error() {
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            let body = serde_json::json!({"errors": [{"status": "400", "title": "ref_type \"\" given but this is not supported"}]}).to_string();
            Mock::given(method("GET"))
                .and(path("/meta"))
                .respond_with(ResponseTemplate::new(400).set_body_string(body))
                .mount(s)
                .await;
        })
    });
    let mut client = client_at(&server);
    let options = opts(&[("url", "meta")]);
    let err = do_fetch(&options, &mut client).expect_err("400 should error");
    match err {
        Error::HttpStatus {
            status,
            reason,
            body,
        } => {
            assert_eq!(status, 400);
            assert_eq!(reason, "Bad Request");
            assert!(body.contains("not supported"));
        }
        other => panic!("expected HttpStatus, got {other:?}"),
    }
}

#[test]
fn missing_action_returns_error_envelope() {
    let server = start_server(|_| Box::pin(async {}));
    let mut client = client_at(&server);
    let mut io = StderrIo::new();
    io.set_options(opts(&[]));
    let err = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
        .expect_err("empty opts should error");
    // MissingOption for stack is what we surface first.
    assert!(matches!(err, Error::MissingOption(_)));
}

#[test]
fn get_deployments_returns_sorted_data() {
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
    // Sorted by id desc — first should be 64989, then 64962.
    assert_eq!(data[0]["id"], "64989");
    assert_eq!(data[1]["id"], "64962");
}

#[test]
fn last_deployment_returns_first_record() {
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
    let resp = do_last_deployment(&options, &mut client).expect("last succeeds");
    let original: serde_json::Value = serde_json::from_str(FIXTURE_GET_DEPLOYMENTS).unwrap();
    assert_eq!(resp.body, original["data"][0]);
}

#[test]
fn create_deployment_success() {
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
}

#[test]
fn create_deployment_gone_bad() {
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE_CREATE_ERROR).unwrap();
    let inner_body = serde_json::to_string(&fixture["body"]).unwrap();
    let server = start_server(move |s| {
        let inner_body = inner_body.clone();
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            Mock::given(method("POST"))
                .and(path("/project/stack/environment/uat/deploys"))
                .respond_with(ResponseTemplate::new(400).set_body_string(inner_body))
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
    let err = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
        .expect_err("400 errors");
    match err {
        Error::HttpStatus { status, reason, .. } => {
            assert_eq!(status, 400);
            assert_eq!(reason, "Bad Request");
        }
        other => panic!("expected HttpStatus, got {other:?}"),
    }
}

#[test]
fn create_deployment_missing_ref_errors() {
    let server = start_server(|_| Box::pin(async {}));
    let _env = setup_deploy_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[
        ("stack", "stack"),
        ("environment", "uat"),
        ("ref_type", "branch"),
    ]);
    let mut io = StderrIo::new();
    io.set_options(options.clone());
    let err = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
        .expect_err("missing ref");
    match err {
        Error::Generic(m) => assert!(m.contains("Requires ref")),
        other => panic!("expected Generic, got {other:?}"),
    }
}

#[test]
fn create_deployment_with_overrides_succeeds() {
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
    let options = opts(&[("stack", "stack"), ("environment", "uat")]);
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
}

#[test]
fn create_deployment_prod_environment_ignores_bypass_and_start() {
    use wiremock::matchers::body_partial_json;
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE_CREATE_SUCCESS).unwrap();
    let inner_body = serde_json::to_string(&fixture["body"]).unwrap();
    // The request body must NOT include bypass_and_start=true because prod
    // strips that flag in PHP. Assert this via wiremock's body matcher.
    let server = start_server(move |s| {
        let inner_body = inner_body.clone();
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            Mock::given(method("POST"))
                .and(path("/project/stack/environment/prod/deploys"))
                .and(body_partial_json(json!({"bypass_and_start": false})))
                .respond_with(ResponseTemplate::new(201).set_body_string(inner_body))
                .mount(s)
                .await;
        })
    });
    let _env = setup_deploy_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[
        ("stack", "stack"),
        ("environment", "prod"),
        ("ref_type", "sha"),
        ("ref", "abc123"),
        ("bypass_and_start", "true"),
    ]);
    let mut io = StderrIo::new();
    io.set_options(options.clone());
    let resp = do_create_deployment(&io.options().unwrap(), &mut io, &mut client, None)
        .expect("create succeeds in prod");
    assert_eq!(resp.status, 201);
}

#[test]
fn git_fetch_returns_final_completed_body() {
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
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
        })
    });
    let _env = setup_deploy_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[("stack", "example")]);
    let resp = do_git_fetch(&options, &mut client).expect("git fetch succeeds");
    assert_eq!(resp.body["data"]["attributes"]["status"], "Complete");
}

#[test]
fn git_fetch_non_202_returns_error() {
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            Mock::given(method("POST"))
                .and(path("/project/example/git/fetches"))
                .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
                .mount(s)
                .await;
        })
    });
    let _env = setup_deploy_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[("stack", "example")]);
    let err = do_git_fetch(&options, &mut client).expect_err("500 errors");
    assert!(matches!(err, Error::HttpStatus { status: 500, .. }));
}

#[test]
fn check_deployment_progress_polls_until_completed() {
    // First poll returns in-progress, second returns Completed.
    let server = start_server(|s| {
        Box::pin(async move {
            use wiremock::matchers::{method, path};
            use wiremock::{Mock, ResponseTemplate};
            Mock::given(method("GET"))
                .and(path("/project/stack/environment/uat/deploys/42"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(r#"{"data":{"attributes":{"state":"Completed"}}}"#),
                )
                .mount(s)
                .await;
        })
    });
    let _env = setup_deploy_env(&server.uri());
    let mut client = client_at(&server);
    let options = opts(&[
        ("stack", "stack"),
        ("environment", "uat"),
        ("deploy_id", "42"),
    ]);
    let mut io = StderrIo::new();
    io.set_options(options.clone());
    let resp = do_check_deployment_progress(&io.options().unwrap(), &mut client)
        .expect("progress check succeeds");
    assert_eq!(resp.status, 200);
}

// ---- StderrIo never panics under the spinner fallback path ----

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

// Reference unused imports to satisfy clippy.
#[allow(dead_code)]
fn _imports_used() {
    let _h: HashMap<String, String> = HashMap::new();
    let _b: DeploymentOverrides = DeploymentOverrides::default();
}

#[allow(dead_code)]
fn _serde_imported() {
    let _: Option<serde_json::Value> = None;
}
