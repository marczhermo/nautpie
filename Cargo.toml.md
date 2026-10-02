# `Cargo.toml` — Project Manifest

This file is the **manifest** for the `nautpie` crate. Cargo reads it to figure out:

- What the crate is called and what version it is.
- What edition of Rust it targets.
- What other crates it depends on.
- What binary and library entry points to build.
- What lints to enforce.

This is the file you'd edit if you wanted to **add a new dependency**, **bump the version**, or **change the lint policy**.

---

## The Big Picture

The file is divided into sections, each introduced by a `[section-name]` header:

| Section | What it does |
|---|---|
| `[package]` | Crate metadata: name, version, edition, MSRV. |
| `[[bin]]` | Declares a binary target (the `nautpie` executable). |
| `[lib]` | Declares the library target. |
| `[dependencies]` | Runtime dependencies. |
| `[dev-dependencies]` | Test and example-only dependencies. |
| `[lints.clippy]` | Centralised clippy policy (added per Azure guidelines). |
| `[lints.rust]` | Centralised `rustc` lint policy. |

---

## Walkthrough

### `[package]`

```toml
[package]
name = "nautpie"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
description = "DeployNaut / Bitbucket Pipelines deployment client (Rust port of nautpie.phar)"
license = "MIT"
```

**Teaching points:**

- **`name`** — the crate name. Used when other crates depend on this one (`use nautpie::...`).
- **`version = "0.1.0"`** — follows [Semantic Versioning](https://semver.org/). The leading `0` means "pre-1.0, anything goes." A `1.0` release would signal API stability.
- **`edition = "2021"`** — which *edition* of Rust this crate targets. Editions are opt-in language versions that allow breaking changes without bumping the major version. `2021` is the third stable edition (after 2015 and 2018).
- **`rust-version = "1.85"`** — the **MSRV** (Minimum Supported Rust Version). The crate promises to compile on any Rust version `>= 1.85`. CI must verify this. The Azure SDK guidelines require MSRV to be no more than 6 months old at release time.
- **`description` and `license`** — informational. Cargo uses them when publishing to crates.io.

### Binary and library targets

```toml
[[bin]]
name = "nautpie"
path = "src/main.rs"

[lib]
name = "nautpie"
path = "src/lib.rs"
```

**Teaching points:**

- **`[[bin]]` (double brackets)** — declares a *list* of binaries. Even though there's only one, the double brackets are required syntax.
- **`[lib]` (single brackets)** — declares a single library. Most crates have at most one library.
- **`path`** — points to the source file. Defaults to `src/main.rs` and `src/lib.rs`, so these lines could be omitted. They're explicit here for clarity.
- **Same name for bin and lib.** This is allowed and even useful: when another crate depends on `nautpie`, it gets the library; when users install `cargo install nautpie`, they get the binary.

### Dependencies

```toml
[dependencies]
clap = { version = "4.6", features = ["derive"] }
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
owo-colors = { version = "4", features = ["supports-colors"] }
dotenvy = "0.15"
reqwest = { version = "0.13", default-features = false, features = ["blocking", "json", "rustls"] }
base64 = "0.22"
url = "2"
chrono = "0.4"
ratatui = "0.29"
crossterm = "0.28"
```

**Teaching points:**

- **`version = "x.y"`** — the version requirement. `4.6` means `>=4.6.0, <5.0.0` (semver caret requirement, the default).
- **`features = ["derive"]`** — enable optional features for this crate. `clap`'s `derive` feature enables `#[derive(Parser)]` and similar.
- **`default-features = false`** — opt out of `reqwest`'s default features (which include `native-tls`, `default-tls`, etc.). We instead enable only `blocking`, `json`, and `rustls`. This produces a smaller binary with no native TLS dependency.
- **`reqwest = { version = "0.13", ..., features = ["blocking"] }`** — we deliberately use the **blocking** flavor of `reqwest` because the binary is sync (matches PHP parity). Async `reqwest` would require a tokio runtime.

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
wiremock = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**Teaching points:**

- **`[dev-dependencies]`** — only used for tests, examples, and benchmarks. Not included in the production binary.
- **`assert_cmd`** — runs the compiled binary as a subprocess and asserts on its output. Used in `tests/cli_smoke.rs`.
- **`predicates`** — composable assertion matchers (`predicate::str::contains(...)`).
- **`wiremock`** — in-process HTTP server for testing HTTP clients without a real network.
- **`tokio`** — needed by `wiremock` for async setup. We don't use it in production code.

### Lint policy (Azure SDK guideline)

```toml
# Centralised lint policy. Mirrors what CI runs (`cargo clippy -D warnings`).
# Keeping it in Cargo.toml means `cargo clippy` locally uses the same rules.
[lints.clippy]
# We allow these pedantic lints because they produce too much noise on a
# CLI tool of this size. Each one has been considered; loosening them here
# keeps the clippy output actionable instead of full of false positives.
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
similar_names = "allow"

# Rust lints: enforce documentation completeness on every public item.
[lints.rust]
missing_docs = "warn"
```

**Teaching points:**

- **`[lints.clippy]`** — a top-level table that controls clippy's behaviour. Each entry is a lint name and a level (`allow`, `warn`, `deny`).
- **`module_name_repetitions = "allow"`** — overrides a default-on pedantic lint that would flag `deployment_details::DeploymentDetails` (the module name repeated in the type). We have several such repetitions in this codebase and don't find them problematic.
- **`must_use_candidate = "allow"`** — silences a lint that flags functions whose return values should be used. For a CLI, this fires too often to be useful.
- **`missing_errors_doc = "allow"`** — the `missing_docs` lint would otherwise require every `Result`-returning function to document which error variants it can produce. This is enforced at a basic level (the function has a `# Errors` section where it matters) but we don't want it blocking the build.
- **`missing_panics_doc = "allow"`** — similar, but for `panic!` calls. We have only one (the `expect` in `main.rs`) and it's documented.
- **`similar_names = "allow"`** — silences a lint that flags near-duplicate identifiers. Useful when you have `Io` and `Io` and `IO` in the same module.
- **`[lints.rust]`** — controls built-in `rustc` lints. The only entry is `missing_docs = "warn"`, which is the Azure guideline `[rust-documentation-warn-missing-docs]`.
- **Why "warn" instead of "deny"?** A `warn` is visible to developers without breaking the build. We could promote it to `deny` once we confirm no existing public items need attention.

---

## Why Is It Written This Way?

- **Explicit over implicit.** Listing every dependency, target, and lint in the manifest makes the build **reproducible**: another developer (or CI) gets exactly the same setup.
- **MSRV declared.** Cargo's resolver will refuse to build with a Rust version older than `1.85`. This satisfies the Azure guideline `[rust-platform-msrv]`.
- **Feature flags trimmed.** Disabling `reqwest`'s default features and re-enabling only what we need (blocking + rustls) keeps the binary lean and avoids pulling in native TLS code.
- **Dev-dependencies isolated.** `wiremock` and `assert_cmd` are not compiled into the release binary — they're only linked when running tests.
- **Centralised lints.** The lint policy lives in `Cargo.toml` (not just in CI commands) so that `cargo clippy` locally matches what CI enforces. This is one of the easier Azure guidelines to satisfy.

---

## Key Rust / Cargo Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `[package]` | top | Crate metadata |
| `[[bin]]` vs `[lib]` | targets | Brackets tell Cargo which kind of target |
| `version = "x.y"` | deps | Semver requirement; `^x.y` is the default |
| `features = [...]` | deps | Enable optional functionality on a crate |
| `default-features = false` | reqwest | Opt out of defaults; only enable what we list |
| `rust-version` | package | MSRV promise |
| `[dependencies]` | runtime | Crates linked into the final binary |
| `[dev-dependencies]` | tests/examples | Crates only linked for `cargo test` and `cargo run --example` |
| `[lints.clippy]` | lint policy | Centralised clippy configuration |
| `[lints.rust]` | lint policy | Centralised rustc lint configuration |
| `edition` | package | Which Rust edition's syntax to use |
| `description`, `license` | package | Used by crates.io when publishing |
