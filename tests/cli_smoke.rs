//! End-to-end smoke tests for the binary. Runs the compiled `nautpie`
//! binary against the most basic scenarios documented in the README:
//!
//! - `--help` lists both subcommands.
//! - `deploy:naut sampleSuccess` exits 0 and prints a single JSON line
//!   whose body contains the documented success message.
//! - `deploy:naut sampleFail` exits 1 and prints a JSON line whose body
//!   contains the documented failure message.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_both_subcommands() {
    Command::cargo_bin("nautpie")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("deploy:naut"))
        .stdout(predicate::str::contains("ci:bitbucket"));
}

#[test]
fn sample_success_exits_zero_and_emits_json_envelope() {
    let output = Command::cargo_bin("nautpie")
        .unwrap()
        .args(["deploy:naut", "sampleSuccess"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout must be a single JSON line: {stdout:?}"
    );

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(value["status"], 200);
    assert_eq!(value["reason"], "OK");
    assert_eq!(value["body"], "[Action:Success] Response successful.");
}

#[test]
fn sample_fail_exits_one_and_emits_error_envelope() {
    let output = Command::cargo_bin("nautpie")
        .unwrap()
        .args(["deploy:naut", "sampleFail"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout must be a single JSON line: {stdout:?}"
    );

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(value["status"], 1);
    assert_eq!(value["reason"], "Bad Request");
    let body = value["body"].as_str().expect("body should be a string");
    assert!(body.contains("[Action:Fail] Has failed."));
}

#[test]
fn missing_action_returns_error_envelope() {
    let output = Command::cargo_bin("nautpie")
        .unwrap()
        .args(["deploy:naut", "createDeployment"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(value["status"], 1);
    assert_eq!(value["reason"], "Bad Request");
}

#[test]
fn sample_success_accepts_case_insensitive_action() {
    // PHP-style dispatch: `Sample-Success`, `SAMPLE_SUCCESS`, and
    // `samplesuccess` should all route to the success sample action.
    for variant in ["Sample-Success", "SAMPLE_SUCCESS", "samplesuccess"] {
        let output = Command::cargo_bin("nautpie")
            .unwrap()
            .args(["deploy:naut", variant])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let stdout = String::from_utf8(output).expect("stdout utf-8");
        let value: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
        assert_eq!(value["status"], 200, "variant: {variant}");
        assert_eq!(value["body"], "[Action:Success] Response successful.");
    }
}
