# `tests/cli_smoke.rs` — End-to-End Binary Smoke Tests

This file contains **integration tests** that run the actual `nautpie` binary as a subprocess. Unlike unit tests (which live inside the source files in `#[cfg(test)] mod tests`), integration tests live in the `tests/` directory and treat the crate as an **external consumer**.

---

## The Big Picture

These tests verify the **most fundamental contract** of the binary:

1. `--help` lists both subcommands.
2. `sampleSuccess` exits 0 with the documented JSON envelope.
3. `sampleFail` exits 1 with the documented JSON envelope.
4. Missing required options produce a clean error envelope (not a panic).
5. Case-insensitive action dispatch works.

If these five tests pass, the binary is at least minimally functional.

---

## Walkthrough

### Imports

```rust
use assert_cmd::Command;
use predicates::prelude::*;
```

- **`assert_cmd::Command`** — a wrapper around `std::process::Command` that makes it easy to run a binary and assert on its output. We use `Command::cargo_bin("nautpie")` to find and run the compiled binary in the workspace.
- **`predicates::prelude::*`** — a crate that provides **composable assertions**. `predicate::str::contains(...)` is a predicate that checks whether a string contains a substring.

### Test 1: `--help`

```rust
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
```

**Teaching points:**

- **`Command::cargo_bin("nautpie")`** — finds the compiled `nautpie` binary in `target/debug/`. `cargo test` builds the binary before running integration tests, so this always exists when tests run.
- **`.unwrap()`** — panics if the binary can't be located. In practice this never fails for in-tree tests.
- **`.arg("--help")`** — adds a CLI argument. Chainable.
- **`.assert()`** — runs the command and returns an `Assert` object. From here on, every method call asserts something about the result.
- **`.success()`** — asserts the exit code was 0.
- **`.stdout(predicate::str::contains("deploy:naut"))`** — asserts that stdout contains the literal string `"deploy:naut"`. The double `.stdout(...)` calls accumulate multiple assertions on stdout.

### Test 2: success envelope

```rust
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
```

This is the most thorough test in the file. It verifies:

1. The binary exits 0.
2. Stdout has **exactly one line** (the JSON-line contract).
3. The JSON parses cleanly.
4. The status is 200.
5. The reason is `"OK"`.
6. The body matches the documented success message.

**Teaching points:**

- **`.args(["deploy:naut", "sampleSuccess"])`** — pass multiple arguments as an array.
- **`.get_output().stdout.clone()`** — `get_output()` returns an `Output` struct (containing `stdout`, `stderr`, status). Cloning the `stdout` bytes is necessary because `Output` is borrowed by `Assert`.
- **`String::from_utf8(output).expect("stdout utf-8")`** — convert raw bytes to a UTF-8 `String`. `expect` panics with the given message if conversion fails.
- **`stdout.lines().count()`** — count newline-separated lines. The strict `== 1` check enforces the contract.
- **`serde_json::from_str(stdout.trim())`** — parse the trimmed stdout as JSON. `trim()` removes the trailing newline.
- **`value["status"]`** — JSON object indexing by string key. The result is a `Value` that compares with `assert_eq!`.

### Test 3: failure envelope

```rust
#[test]
fn sample_fail_exits_one_and_emits_error_envelope() {
    let output = Command::cargo_bin("nautpie")
        .unwrap()
        .args(["deploy:naut", "sampleFail"])
        .assert()
        .failure()        // exit code is non-zero
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf-8");
    assert_eq!(stdout.lines().count(), 1, ...);

    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect(...);
    assert_eq!(value["status"], 1);
    assert_eq!(value["reason"], "Bad Request");
    let body = value["body"].as_str().expect("body should be a string");
    assert!(body.contains("[Action:Fail] Has failed."));
}
```

The mirror image of test 2, but for the failure path:

- **`.failure()`** — asserts the exit code was non-zero.
- **`value["body"].as_str()`** — `as_str()` returns `Option<&str>`. `.expect("body should be a string")` panics if the body is not a string.
- **`body.contains(...)`** — substring check (PHP `str_contains` style).

### Test 4: missing required options

```rust
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
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect(...);
    assert_eq!(value["status"], 1);
    assert_eq!(value["reason"], "Bad Request");
}
```

Verifies that `createDeployment` without `--stack` or `--environment` produces a JSON error envelope (not a panic, not a stack trace on stderr).

### Test 5: case-insensitive dispatch

```rust
#[test]
fn sample_success_accepts_case_insensitive_action() {
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
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect(...);
        assert_eq!(value["status"], 200, "variant: {variant}");
        assert_eq!(value["body"], "[Action:Success] Response successful.");
    }
}
```

This is the **PHP parity test**. The original PHP version accepted `sampleSuccess`, `SampleSuccess`, and even `SAMPLE_SUCCESS` interchangeably (because PHP's method dispatch is case-insensitive). The Rust port uses `normalise_action()` to convert all of these to `sampleSuccess`.

**Teaching points:**

- **The `for variant in [...]` loop** — runs the same assertions three times with different inputs.
- **`assert_eq!(value["status"], 200, "variant: {variant}")`** — the optional third argument is a format string used in the panic message. This makes test failures easier to diagnose.

---

## Why Is It Written This Way?

- **End-to-end tests for the contract.** These tests verify the **observable behavior** of the binary — what's printed, what exit code is returned. They don't care about internals.
- **`assert_cmd` over raw `std::process::Command`.** `assert_cmd` adds ergonomic assertion chaining (`.success()`, `.stdout(...)`) and handles the cargo binary lookup automatically.
- **Strict one-line stdout check.** The binary's contract with downstream CI scripts is "exactly one JSON line." Verifying this in tests prevents regressions where someone accidentally adds an extra `println!` in `main.rs`.
- **JSON parsing in tests.** Rather than comparing the raw stdout string (which would be brittle to key ordering and whitespace), the tests parse it as JSON and compare specific fields.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `#[test]` | every test | Marks a function as a test |
| `Command::cargo_bin("nautpie")` | every test | Locate the compiled binary |
| `.arg(...)` / `.args(...)` | every test | Add CLI arguments |
| `.assert()` | every test | Run the command and return an `Assert` |
| `.success()` | success tests | Assert exit code 0 |
| `.failure()` | failure tests | Assert exit code non-zero |
| `.stdout(predicate)` | help test | Assert on stdout content |
| `.get_output().stdout` | several tests | Get the captured stdout bytes |
| `String::from_utf8(...)` | every test | Convert bytes to a UTF-8 string |
| `.expect(msg)` | every test | Panic with message on `Err` |
| `.lines().count()` | envelope tests | Count newline-separated lines |
| `serde_json::from_str(...)` | envelope tests | Parse a string as JSON |
| `value["status"]` | envelope tests | Index into a JSON value |
| `.as_str()` | failure test | Downcast a JSON value to `&str` |
| `.contains(...)` | failure test | Substring check |
| `for variant in [...]` | case test | Iterate over test inputs |
| Assert format string | case test | Custom panic message with the failing variant |
| `tests/` directory | whole file | Integration tests live here (not in `src/`) |
