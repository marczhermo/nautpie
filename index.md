# NautPie: A Rust Learning Journey

Welcome to a guided tour of the `nautpie` codebase, organised **by Rust concept rather than by file**. Each chapter is a self-contained lesson that introduces a new idea and points you at the real code that uses it.

If you're brand new to Rust, read the chapters in order. If you've used Rust before and want to see how this codebase handles a specific topic, jump to the chapter you need.

---

## How to Read This Guide

Each chapter has four sections:

1. **What you'll learn** — the Rust concepts introduced.
2. **Why this matters** — when you'd use these patterns in real code.
3. **Read the code** — links to the beginner-friendly docs for the relevant files.
4. **Key concepts** — a cheat sheet of the syntax and idioms introduced.

The **companion docs** (linked throughout) live next to the source files they describe: `src/main.md` is alongside `src/main.rs`, `tests/cli_smoke.md` is alongside `tests/cli_smoke.rs`, and so on.

---

## Table of Contents

### Part I — Foundations

- [**Chapter 1: Project Layout and the Entry Point**](./index.md#chapter-1-project-layout-and-the-entry-point) — what a Rust crate is, how binaries and libraries differ, what `fn main()` does.
- [**Chapter 2: Primitive Types, Pattern Matching, and `Option`**](./index.md#chapter-2-primitive-types-pattern-matching-and-option) — `bool`, `&str`, `String`, `match`, the role of `Option<T>`.

### Part II — Core Language Features

- [**Chapter 3: Error Handling with `Result` and `thiserror`**](./index.md#chapter-3-error-handling-with-result-and-thiserror) — modelling failure as data, the `?` operator, error enums.
- [**Chapter 4: Data Structures and the Builder Pattern**](./index.md#chapter-4-data-structures-and-the-builder-pattern) — `struct`, `HashMap`, the consuming builder, `Option<Vec<T>>`.

### Part III — Polymorphism and Reuse

- [**Chapter 5: Traits, Dynamic Dispatch, and Dependency Injection**](./index.md#chapter-5-traits-dynamic-dispatch-and-dependency-injection) — defining a trait, implementing it, `dyn Trait`, default methods.
- [**Chapter 6: Borrowing, Mutable State, and Closures**](./index.md#chapter-6-borrowing-mutable-state-and-closures) — `&mut`, interior mutability, `Fn`/`FnMut`/`FnOnce`, `move` closures.

### Part IV — Building Real Applications

- [**Chapter 7: Command-Line Parsing with `clap`**](./index.md#chapter-7-command-line-parsing-with-clap) — derive macros, subcommands, action normalisation.
- [**Chapter 8: HTTP, Async Concepts, and Concurrency**](./index.md#chapter-8-http-async-concepts-and-concurrency) — `reqwest` blocking client, traits over async boundaries, polling loops.

### Part V — Composition

- [**Chapter 9: Putting It All Together — The Action Functions**](./index.md#chapter-9-putting-it-all-together--the-action-functions) — how the trait, error, and HTTP layers combine to handle real requests.

### Part VI — Quality and Operations

- [**Chapter 10: Testing — Unit, Integration, and HTTP Mocks**](./index.md#chapter-10-testing--unit-integration-and-http-mocks) — `#[test]`, `assert_cmd`, `wiremock`, env-var locking.
- [**Chapter 11: Documentation as Code**](./index.md#chapter-11-documentation-as-code) — `///` doc comments, doctests, `#![warn(missing_docs)]`.
- [**Chapter 12: Library Design and the Azure SDK Guidelines**](./index.md#chapter-12-library-design-and-the-azure-sdk-guidelines) — what we kept, what we changed, what we didn't.

---

## Chapter 1: Project Layout and the Entry Point

**What you'll learn**

- What a Cargo project is and what `Cargo.toml` describes.
- The difference between a **binary** (`src/main.rs`) and a **library** (`src/lib.rs`).
- What `fn main()` does and why every Rust binary needs one.
- The `#![doc = include_str!(...)]` and `#![warn(missing_docs)]` crate-level attributes.

**Why this matters**

Every Rust project you ever open will have this structure. Understanding the bin/lib split is foundational because it explains why a `Cargo.toml` can have *both* a `[[bin]]` and a `[lib]` section, why `cargo build` produces a binary, and why `cargo test` can `use your_crate::...` even though the binary and the library share a source tree.

**Read the code**

- [`Cargo.toml.md`](./Cargo.toml.md) — the project manifest, including the centralised lint policy.
- [`src/lib.md`](./src/lib.md) — what `lib.rs` does, the `#![doc = include_str!]` trick, and how `pub use` shortens consumer code.
- [`src/main.md`](./src/main.md) — what `main.rs` does, including the `dispatch`/`finish` split that gives every code path a single exit point.

**Key concepts**

| Concept | What it does |
|---|---|
| `Cargo.toml` | The manifest: name, version, deps, targets, lint policy. |
| `[[bin]]` | Declares a binary target (double brackets = list). |
| `[lib]` | Declares the library target (single brackets = single). |
| `fn main()` | Required entry point of every Rust binary. |
| `pub mod foo;` | Declares a public module wired to `src/foo.rs` or `src/foo/mod.rs`. |
| `pub use foo::Bar;` | Re-exports `Bar` from the top of the crate. |
| `#![doc = "..."]` | Sets the crate-level doc comment. |
| `#![warn(lint)]` | Enables a lint for the whole crate. |

---

## Chapter 2: Primitive Types, Pattern Matching, and `Option`

**What you'll learn**

- The difference between `&str` (borrowed string slice) and `String` (owned, heap-allocated).
- How `bool`, `u16`, `i64`, `Duration` are used throughout the codebase.
- `Option<T>` as Rust's way of representing "some value or no value."
- Pattern matching with `match`, including **match guards** (`if condition` after the pattern).
- The `matches!` macro for single-line pattern checks.
- Composing `Option` operations with `.map()`, `.and_then()`, `.ok_or_else()`.

**Why this matters**

Rust has no `null` and no exceptions. `Option<T>` and `Result<T, E>` (covered in the next chapter) are how Rust models "missing" and "failed." Mastering these two enums — and the pattern-matching syntax that goes with them — is the single biggest unlock for reading any Rust code.

**Read the code**

- [`src/boolean.md`](./src/boolean.md) — A small but teaching-dense file. Demonstrates `match` with guards, `Option<&str>` as input, `Result<bool, Error>` as output, the `matches!` macro, and `str::trim`/`str::to_ascii_lowercase` passed as function references.
- [`src/env.md`](./src/env.md) — Shows the more common `Result<T, E>` shape, the `?` operator, and `Vec::with_capacity` for pre-allocated vectors.

**Key concepts**

| Concept | What it does |
|---|---|
| `&str` vs `String` | `&str` is borrowed, immutable, often a literal. `String` is owned, growable, on the heap. |
| `Option<T>` | `Some(value)` or `None`. The replacement for `null`. |
| `match expr { pat => body }` | Exhaustive pattern matching — every possible value must be covered. |
| Match guard `if condition` | Refines a pattern with an extra condition. |
| `matches!(expr, pattern)` | One-line "does this match the pattern?" boolean check. |
| `.map(closure)` | Transform inside `Option` (or `Result`), leave `None`/`Err` alone. |
| `.and_then(closure)` | Chain `Option`s — the closure returns another `Option`. |
| `.ok_or(err)` | Convert `Option<T>` to `Result<T, E>`. |
| `.ok_or_else(\|\| err)` | Lazy version — the closure only runs on `None`. |
| `?` operator | Early-return on `Err`/`None`. |

---

## Chapter 3: Error Handling with `Result` and `thiserror`

**What you'll learn**

- Why Rust uses **typed errors** (enums) instead of exceptions.
- How `thiserror` generates `Display` and `std::error::Error` implementations from `#[derive(Error)]`.
- The difference between **tuple variants** (`MissingEnv(String)`) and **struct variants** (`HttpStatus { status, reason, body }`).
- Building a JSON envelope (`ApiResponse`) that wraps both success and failure in the same shape.
- The `unwrap_or_else` fallback pattern.

**Why this matters**

The contract between a CLI and its callers is encoded in the error type. By making `Error` an enum, the code can pattern-match on the specific kind of failure (`Error::MissingEnv(_)` vs `Error::Timeout(_)` vs `Error::HttpStatus { .. }`) and produce different responses. This is more powerful than stringly-typed errors but less flexible than exceptions — a deliberate trade-off that pays off in tests and refactors.

**Read the code**

- [`src/error.md`](./src/error.md) — The `Error` enum and `ApiResponse` envelope. Eight error variants, each with a `#[error("...")]` attribute that controls its `Display` output.

**Key concepts**

| Concept | What it does |
|---|---|
| `enum` with data | Each variant can carry zero, one, or many fields. |
| Tuple variant `Variant(T)` | Concise, unnamed field. |
| Struct variant `Variant { field: T }` | Self-documenting, named fields. |
| `#[derive(thiserror::Error)]` | Generates `Display` and `std::error::Error` impls. |
| `#[error("text {field}")]` | Sets the `Display` format string for a variant. |
| `serde_json::Value` | An enum that can hold any JSON value (null, bool, number, string, array, object). |
| `Result<T, E>` | `Ok(T)` for success, `Err(E)` for failure. |
| `unwrap_or_else(\|\| default)` | "If this fails, compute and use this fallback." |

---

## Chapter 4: Data Structures and the Builder Pattern

**What you'll learn**

- The anatomy of a Rust `struct`: named fields, derives, the difference between `pub` fields and `pub(crate)` fields.
- Encapsulation via private fields + public methods.
- The **consuming builder pattern**: methods that take `mut self` and return `Self`.
- Why `impl Into<String>` is more flexible than `String` for builder parameters.
- The `raw identifier` syntax `r#name` for using reserved keywords as field names.
- Manual `Default` impls vs `#[derive(Default)]`.

**Why this matters**

`DeploymentDetails` is the only domain model in the codebase. It mirrors a PHP class with chained setters and demonstrates how Rust's strict ownership rules turn a "set a field, return the object" pattern into a clean, allocation-free builder. Every data-heavy Rust crate you'll see uses some variant of this.

**Read the code**

- [`src/options.md`](./src/options.md) — A small wrapper around `HashMap<String, String>`. Demonstrates the value of private fields, `impl Into<String>` flexibility, and a `Clone` derive.
- [`src/deployment_details.md`](./src/deployment_details.md) — The big one. Consuming builder, raw identifier `r#ref`, manual `Default` impl, custom JSON serialisation in `values()`, and the PHP-parity quirk of dropping empty fields.

**Key concepts**

| Concept | What it does |
|---|---|
| `pub struct Foo { ... }` | Define a public struct. |
| `pub field: T` vs `field: T` | Public field vs private field. Private fields enforce the encapsulation boundary. |
| `#[derive(Debug, Clone, Default)]` | Auto-generate trait impls. |
| `impl Default for Foo { fn default() -> Self { ... } }` | Manual `Default` when fields don't have natural defaults. |
| `mut self` in builder methods | Take ownership so we can modify and return. |
| `impl Into<String>` | Accepts `&str`, `String`, `format!(...)` — anything convertible. |
| `.into()` | Performs the `Into` conversion (type-inferred at the call site). |
| `r#ref` | Raw identifier — escapes a reserved keyword. |
| `impl AsRef<str>` | "I accept anything that can be borrowed as `&str`." |
| `.as_ref()` | Borrow as the target type. |

---

## Chapter 5: Traits, Dynamic Dispatch, and Dependency Injection

**What you'll learn**

- How a **trait** is Rust's answer to interfaces or abstract base classes.
- The difference between **static dispatch** (generics) and **dynamic dispatch** (`dyn Trait`).
- How trait objects (`Box<dyn Trait>`, `&mut dyn Trait`) enable **dependency injection** and testability.
- **Default methods** on traits — a kind of "interface with default implementations."
- How a unit struct (`pub struct Foo;`) can be a tag for a trait implementation.
- The `trait Object: Sized` distinction (informal; not deeply covered here).

**Why this matters**

This codebase has two traits — `Io` and `HttpClient` — and every action function takes them as parameters. The whole reason `wiremock` works in tests is that `HttpClient` is a trait: tests substitute a `wiremock`-backed client, production uses `reqwest`, and the action functions don't care which. This is the Rust equivalent of passing interfaces around, and it's one of the most important patterns in the language.

**Read the code**

- [`src/io/mod.md`](./src/io/mod.md) — The `Io` trait: four required methods plus two with default impls. Discusses the **side-channel pattern** for passing options through a narrow trait.
- [`src/io/stderr_io.md`](./src/io/stderr_io.md) — The production impl. Uses `owo-colors` for TTY-aware coloured output, demonstrates the `if let Some(v) = ...` pattern for optional storage.
- [`src/http/mod.md`](./src/http/mod.md) — The `HttpClient` trait. Discusses **why mutable state** is acceptable here despite Azure's "immutable clients" guidance.
- [`src/commands/mod.md`](./src/commands/mod.md) — The `Command` trait, the unit-struct pattern (`pub struct DeployNaut;`), and how `main.rs` dispatches to either implementor.

**Key concepts**

| Concept | What it does |
|---|---|
| `pub trait Foo { ... }` | Define an interface (set of required methods). |
| `impl Foo for Bar { ... }` | Implement the trait for a concrete type. |
| `&mut dyn Trait` | Mutable reference to a trait object — runtime polymorphism. |
| Default method body | `fn method(&self) { ... }` in the trait itself = free impl. |
| `pub struct Foo;` | Unit struct — just a tag for trait impls and constants. |
| `Box<dyn FnMut(...)>` | Heap-allocated, type-erased, mutable closure. |

---

## Chapter 6: Borrowing, Mutable State, and Closures

**What you'll learn**

- The difference between **borrowing** (`&T`, `&mut T`) and **owning**.
- Why `&mut self` is required for methods that change state.
- The three closure kinds: `Fn` (immutable borrow), `FnMut` (mutable borrow), `FnOnce` (consume).
- The `move` keyword on closures: when it's required, when it isn't.
- `let _ = ...` to silently discard a `Result` you don't care about.
- Locking stdout/stderr to prevent interleaving.

**Why this matters**

Borrowing is Rust's central innovation. The compiler enforces that you can't have two `&mut` references to the same data at once, which prevents data races without a runtime cost. Closures build on top of borrowing: capturing by reference, by mutable reference, or by ownership. Together, these concepts let you write safe concurrent code.

**Read the code**

- [`src/http/reqwest_client.md`](./src/http/reqwest_client.md) — A stateful HTTP client that mutates `endpoint`, `authorization`, and `content_type` over its lifetime. Also demonstrates `Base64` encoding, `HeaderMap` construction, and the `(400..=599).contains(&status)` inclusive range check.
- [`src/tui/spinner.md`](./src/tui/spinner.md) — A polling loop with `FnMut` closures. Teaches the difference between the three closure flavors and when each is appropriate.

**Key concepts**

| Concept | What it does |
|---|---|
| `&T` | Immutable borrow — multiple readers OK. |
| `&mut T` | Mutable borrow — exclusive access. |
| `Fn` / `FnMut` / `FnOnce` | Three closure flavors with increasing exclusivity. |
| `move` on a closure | Force the closure to take ownership of captured variables. |
| `let _ = expr;` | Discard a value (no unused-result warning). |
| `(a..=b).contains(&x)` | Inclusive range check. |
| `instant + Duration` | Compute a future `Instant`. |

---

## Chapter 7: Command-Line Parsing with `clap`

**What you'll learn**

- How `clap`'s `#[derive(Parser)]` generates argument parsers at compile time.
- The difference between `#[command(subcommand)]` and `#[derive(Subcommand)]`.
- How `Option<T>` fields model optional flags.
- Raw identifiers (`r#ref`) for flag names that collide with reserved words.
- The value of **action normalisation** for cross-language compatibility.
- Match-arm dispatch over normalised action names.

**Why this matters**

Almost every Rust CLI you'll write uses `clap`. The derive macros turn struct definitions into a complete parser, with `--help` text generated from doc comments. Understanding this pattern means you can add a subcommand in minutes.

**Read the code**

- [`src/cli.md`](./src/cli.md) — The whole `clap` surface for the binary. Defines `Cli`, `CommandKind`, `DeployNautArgs`, `BitbucketArgs`, and the `normalise_action` helper that lets the user pass `sampleSuccess`, `SampleSuccess`, `sample_success`, etc., interchangeably.

**Key concepts**

| Concept | What it does |
|---|---|
| `#[derive(Parser)]` | Generates `parse()` / `try_parse_from()`. |
| `#[derive(Subcommand)]` | Generates enum-based subcommand parsing. |
| `#[command(name = "deploy:naut")]` | Sets the literal CLI name (overrides the default). |
| `#[arg(long)]` | Mark a field as a long flag (`--foo`). |
| `#[arg(value_name = "ACTION")]` | Custom placeholder name in `--help`. |
| `Option<T>` fields | Optional flags — `None` if not supplied. |
| `r#ref` | Raw identifier for field names that are reserved words. |
| Doc comment `///` | Becomes the `--help` text for that field. |

---

## Chapter 8: HTTP, Async Concepts, and Concurrency

**What you'll learn**

- The difference between sync (`reqwest::blocking`) and async (`reqwest`) clients.
- How a `wiremock` test server runs **inside the test process** and lets you assert on the *outgoing* request shape.
- The **higher-ranked trait bound** `for<'a> FnOnce(&'a MockServer) -> Pin<Box<dyn Future + 'a>>` for async setup on a sync test runner.
- Why `eprintln!` to stderr is used for debug logs (stdout is reserved for the JSON envelope).
- How a polling loop polls an endpoint until a condition is met.
- Inclusive ranges for status-code checks.

**Why this matters**

`reqwest::blocking` is the right choice when you're porting synchronous code or writing a CLI. `wiremock` is the de-facto standard for HTTP client testing in Rust. Together, they let you write fast, deterministic HTTP tests without spinning up a real server.

**Read the code**

- [`src/http/reqwest_client.md`](./src/http/reqwest_client.md) — Production HTTP client. Discusses `Client::builder()`, the `danger_accept_invalid_certs(true)` trade-off, `HeaderMap` construction, and `(400..=599)` error mapping.
- [`tests/http_test.md`](./tests/http_test.md) — Test-side companion. The `start_server` helper isolates the awkward `tokio` runtime boundary.

**Key concepts**

| Concept | What it does |
|---|---|
| `reqwest::blocking::Client` | Synchronous HTTP client (vs the default async). |
| `Client::builder().build()` | Builder pattern for the underlying client. |
| `wiremock::MockServer::start().await` | In-process HTTP server for tests. |
| `Mock::given(method).and(path)` | Compose request matchers with logical AND. |
| `for<'a> FnOnce(&'a ...) -> ...` | Higher-ranked trait bound for async closures. |
| `Pin<Box<dyn Future + 'a>>` | Pinned, heap-allocated, type-erased future. |
| `eprintln!` | Write to stderr. Reserved for status messages; never pollutes stdout. |
| `Duration::from_secs(n)` | Create a duration from seconds. |

---

## Chapter 9: Putting It All Together — The Action Functions

**What you'll learn**

- How the trait, error, and HTTP layers combine in a real action function.
- The "set up env → set up client → build request → send → decode → return envelope" pipeline.
- The `setup_*_env` pattern for shared configuration.
- Cross-module composition (e.g., `do_deploy_package` calling `do_create_deployment`).
- The Bitbucket-specific quirk of tag truncation.

**Why this matters**

Up to this point you've seen pieces in isolation. This chapter shows them assembled. After reading these files you should be able to predict where in the codebase any new action would be added and what it would look like.

**Read the code**

- [`src/commands/sample.md`](./src/commands/sample.md) — The smallest action. Demonstrates the basic `do_*` shape.
- [`src/commands/deploy_naut.md`](./src/commands/deploy_naut.md) — The main API surface. `setup_deploy_naut_env`, `do_fetch`, `do_create_deployment`, `do_git_fetch`, `do_get_deployments`, `do_last_deployment`, `check_deployment_progress`. Discusses the side-channel pattern, the `DeploymentOverrides` struct, and `strtotime_or_now`.
- [`src/commands/bitbucket.md`](./src/commands/bitbucket.md) — The Bitbucket integration. `do_create_access_token`, `do_create_tag` (with the truncation quirk), `do_deploy_package` (which calls back into `deploy_naut`).

**Key concepts**

| Concept | What it does |
|---|---|
| `setup_*_env(http)` | Read env vars, configure the HTTP client, return values. |
| `http.request(method, url, headers, body)` | The single point where HTTP is performed. |
| `decode_body(raw) -> Value` | Helper to parse JSON or return `Value::Null`. |
| Cross-module call | `do_deploy_package` calls `do_create_deployment` directly (not through the dispatcher). |
| `move` closure | `move \|http\| poll_status(...)` for closures that outlive the call site. |
| `Vec::retain(...)` | Like `Array.filter` — keep items where the closure returns `true`. |

---

## Chapter 10: Testing — Unit, Integration, and HTTP Mocks

**What you'll learn**

- The difference between **unit tests** (`#[cfg(test)] mod tests` inside `src/`) and **integration tests** (`tests/*.rs`).
- `cargo test` runs both, plus doctests, plus examples.
- `assert_cmd::Command::cargo_bin("nautpie")` for end-to-end tests of the binary.
- `wiremock` for testing HTTP clients against a fake server.
- The **env-lock pattern** for serialising tests that mutate process-wide env vars.
- `RAII` cleanup via `Drop` impls.
- `assert_cmd` and `predicates` for readable assertions.

**Why this matters**

Tests are a first-class concern in Rust. The build system runs them automatically; the package layout (`tests/` vs `src/`) separates unit and integration tests by convention; and the ecosystem has strong tools (`wiremock`, `assert_cmd`, `predicates`, `tokio-test`) for almost every kind of test you might want to write.

**Read the code**

- [`tests/cli_smoke.md`](./tests/cli_smoke.md) — End-to-end tests that spawn the compiled binary and assert on stdout. Five tests covering `--help`, success, failure, missing options, and case-insensitive dispatch.
- [`tests/http_test.md`](./tests/http_test.md) — Seven tests of the `ReqwestHttpClient` against `wiremock`. Includes the `start_server` helper with its higher-ranked trait bound.
- [`tests/deploy_naut_test.md`](./tests/deploy_naut_test.md) — The biggest test file. Includes the `EnvLock` pattern, the `CaptureIo` custom impl, and assertions about how `bypass_and_start` is stripped for production environments.
- [`tests/bitbucket_test.md`](./tests/bitbucket_test.md) — Tests for the Bitbucket actions. Uses fixtures (`include_str!`) and the same env-lock pattern.

**Key concepts**

| Concept | What it does |
|---|---|
| `#[test]` | Marks a function as a test. |
| `#[cfg(test)]` | "Only compile this in test mode." |
| `tests/` directory | Integration tests live here, one file per behaviour group. |
| `Command::cargo_bin("nautpie")` | Locate and run the compiled binary. |
| `.assert().success()` | Assert the binary exited with code 0. |
| `.assert().failure()` | Assert the binary exited non-zero. |
| `wiremock::MockServer::start().await` | In-process HTTP server. |
| `Mock::given(...).respond_with(...)` | Mount a request matcher + response. |
| `static MUTEX: Mutex<T>` | Global mutex for serialising process-wide state. |
| `impl Drop for Lock` | RAII cleanup on scope exit. |
| `include_str!("path")` | Embed a file's contents as a `&'static str`. |
| `let _env = ...` | Hold an `EnvLock` for its `Drop` side effect. |

---

## Chapter 11: Documentation as Code

**What you'll learn**

- The three flavours of doc comments: `///` (item), `//!` (module), and `#[doc = "..."]` (attribute).
- How `cargo doc` renders doc comments to HTML.
- **Doctests**: code blocks inside doc comments that are compiled and run by `cargo test`.
- The `no_run`, `ignore`, and `should_panic` attributes for doctests.
- `#![warn(missing_docs)]` for enforcing documentation completeness at compile time.
- `#![doc = include_str!("../README.md")]` to embed a README as the crate overview.

**Why this matters**

Documentation that compiles is documentation that **doesn't rot**. Doctests catch signature changes the moment they happen. `#![warn(missing_docs)]` catches missing docs before they reach code review. Together, these three mechanisms turn documentation from a chore into an automatic part of the build.

**Read the code**

- Run `cargo doc --open` in this repo to see the rendered output. Every public `fn`, `struct`, `enum`, and trait has a `///` block.
- [`src/lib.md`](./src/lib.md) — Re-examines the `lib.rs` attributes including the crate-level `//!` comment and the `#![doc = include_str!]` trick.
- [`src/error.md`](./src/error.md) — A model for how to document an enum: every variant has its own `///` block, and the `#[error("...")]` attribute is paired with a prose explanation of when the variant is used.

**Key concepts**

| Concept | What it does |
|---|---|
| `///` | Doc comment for the item below. |
| `//!` | Doc comment for the enclosing module. |
| `#[doc = "..."]` | Doc comment as an attribute (useful for `include_str!`). |
| `cargo doc --open` | Build and open the rendered HTML. |
| `cargo test --doc` | Run doctests only. |
| ` ```rust no_run ` | Compiles but doesn't run. |
| ` ```rust ignore ` | Doesn't even compile. |
| `#![warn(missing_docs)]` | Warn on undocumented public items. |
| `#![doc = include_str!("../README.md")]` | Embed a file as crate-level docs. |

---

## Chapter 12: Library Design and the Azure SDK Guidelines

**What you'll learn**

- What the Azure SDK Rust Guidelines are and which of them apply to a non-Azure CLI.
- How `thiserror`, `cargo clippy -D warnings`, and `#![warn(missing_docs)]` work together.
- The trade-off between **PHP-parity** (preserve the wire format exactly) and **idiomatic Rust** (use the type system more aggressively).
- How to read a guideline audit and pick the items that have the best effort-to-payoff ratio.

**Why this matters**

Real-world Rust code is shaped by guidelines like the Azure SDK's, the Rust API Guidelines, and your team's own conventions. Knowing how to read them — and which to apply — is a transferable skill. This codebase is a worked example of how a small CLI can adopt about a third of a much larger guideline document without rewriting itself.

**Read the code**

- [`.kimchi/docs/azure-sdk-guidelines-review.md`](./.kimchi/docs/azure-sdk-guidelines-review.md) — The full audit. Lists what we kept, what we changed, what's not applicable, and a prioritised action list of the remaining improvements.

**Key concepts**

| Concept | What it does |
|---|---|
| Azure SDK Guidelines | Microsoft-maintained Rust-specific design rules for client libraries. |
| Idiomatic Rust | "Follow general Rust conventions, not translations from another language." |
| Service client | A type users construct directly to talk to a service (Azure style). |
| MSRV | The oldest Rust version the crate promises to compile on. |
| `cargo clippy -D warnings` | Treat all warnings as errors. |
| `[lints.clippy]` | Centralised clippy policy in `Cargo.toml`. |
| `pub fn new(...) -> Result<Self>` | The standard constructor signature for fallible clients. |
| `SafeDebug` | Azure's PII-safe alternative to `Debug`. Not applicable here. |
| Backwards compatibility | "Breaking changes are worse than missing features." |

---

## Appendix A: Reading Order Suggestions

### For a complete Rust beginner

1. Chapter 1 — set up your mental model.
2. Chapter 2 — `Option` and pattern matching are the foundation.
3. Chapter 3 — `Result` and error enums.
4. Chapter 4 — structs and builders.
5. Chapter 5 — traits (the most distinctive Rust feature).
6. Chapter 6 — borrowing and closures (spend time here).
7. Chapter 7 — `clap` is concrete and immediately useful.
8. Chapter 8 — HTTP, but skim on first read.
9. Chapter 9 — read this last among the "core" chapters.
10. Chapter 10 — testing (a small dose is enough to start).
11. Chapter 11 — documentation.
12. Chapter 12 — optional; design philosophy.

### For someone who knows Rust but is new to this codebase

1. Skim Chapter 1, then jump to:
2. Chapter 5 — understand the `Io` and `HttpClient` traits.
3. Chapter 9 — read the action functions end-to-end.
4. Chapter 10 — see how the tests exercise the actions.
5. Chapter 12 — the audit tells you what design decisions were deliberate.

### For a reviewer or maintainer

1. Chapter 12 — the audit summarises every non-obvious decision.
2. Chapter 5 — understand the mutable-client decision before changing it.
3. Chapter 9 — the action functions are the most likely place for new code.

---

## Appendix B: Glossary

The terms used throughout these chapters, defined briefly.

| Term | Definition |
|---|---|
| `crate` | A Rust package — either a binary or a library. |
| `mod` | A module within a crate. Declared with `mod name;`. |
| `trait` | A set of methods a type must implement. Like an interface. |
| `dyn Trait` | A trait object — a value of "some type implementing this trait." |
| `Result<T, E>` | Rust's success-or-failure type: `Ok(T)` or `Err(E)`. |
| `Option<T>` | Rust's "some value or no value" type: `Some(T)` or `None`. |
| `?` | The question-mark operator. Early-return on `Err` or `None`. |
| `&T` / `&mut T` | Immutable / mutable borrow. |
| `String` vs `&str` | Owned growable string vs borrowed immutable slice. |
| `Vec<T>` | Growable array of `T`. |
| `HashMap<K, V>` | Hash table keyed by `K` with values of `V`. |
| `Box<T>` | Heap allocation of `T`. |
| `Arc<T>` | Thread-safe shared ownership of `T`. |
| `async` / `await` | Rust's syntax for asynchronous functions. (Not used in this CLI.) |
| `Send` / `Sync` | Marker traits for thread-safety. |
| `Default` | A trait with a `default()` method. |
| `Clone` | A trait with a `clone()` method. |
| `Debug` | A trait with a `{:?}` formatter for debugging. |
| `PartialEq` / `Eq` | Traits for equality comparison. |
| `#[derive(...)]` | Auto-implement a set of traits. |
| `cargo build` | Compile the crate. |
| `cargo test` | Run all tests. |
| `cargo doc` | Generate documentation. |
| `cargo clippy` | Run the linter. |
| `cargo fmt` | Auto-format the code. |

---

## Appendix C: Companion Document Index

For quick lookup, here is every doc file in this repository, grouped by chapter.

### Part I — Foundations

- [`Cargo.toml.md`](./Cargo.toml.md)
- [`src/lib.md`](./src/lib.md)
- [`src/main.md`](./src/main.md)
- [`src/boolean.md`](./src/boolean.md)
- [`src/env.md`](./src/env.md)

### Part II — Core Language

- [`src/error.md`](./src/error.md)
- [`src/options.md`](./src/options.md)
- [`src/deployment_details.md`](./src/deployment_details.md)

### Part III — Polymorphism

- [`src/io/mod.md`](./src/io/mod.md)
- [`src/io/stderr_io.md`](./src/io/stderr_io.md)
- [`src/http/mod.md`](./src/http/mod.md)
- [`src/commands/mod.md`](./src/commands/mod.md)
- [`src/http/reqwest_client.md`](./src/http/reqwest_client.md)
- [`src/tui/spinner.md`](./src/tui/spinner.md)

### Part IV — Applications

- [`src/cli.md`](./src/cli.md)
- [`src/commands/sample.md`](./src/commands/sample.md)
- [`src/commands/deploy_naut.md`](./src/commands/deploy_naut.md)
- [`src/commands/bitbucket.md`](./src/commands/bitbucket.md)

### Part V — Quality

- [`tests/cli_smoke.md`](./tests/cli_smoke.md)
- [`tests/http_test.md`](./tests/http_test.md)
- [`tests/deploy_naut_test.md`](./tests/deploy_naut_test.md)
- [`tests/bitbucket_test.md`](./tests/bitbucket_test.md)
- [`examples/README.md`](./examples/README.md)
- [`.kimchi/docs/azure-sdk-guidelines-review.md`](./.kimchi/docs/azure-sdk-guidelines-review.md)

---

Happy learning! If something doesn't make sense, the best fix is to read the relevant `src/*.md` doc — every doc has a "Why is it written this way?" section that captures the reasoning, not just the mechanics.
