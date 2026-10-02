# `examples/` — Runnable Code Snippets

This directory contains **runnable examples** that demonstrate how to use `nautpie` as a library. The Azure SDK guidelines recommend providing one example per common use case (`[rust-repo-samples-examples]`), and these three examples cover the three most useful entry points.

---

## The Big Picture

Cargo treats any `.rs` file in the `examples/` directory as a separate program. Each one is compiled and runnable via:

```sh
cargo run --example <name>
```

The example file **must** contain a `fn main()` because Cargo compiles it as a standalone binary. The examples use `nautpie` as a regular dependency — they import from the crate via `use nautpie::...` just like an external consumer would.

---

## The Three Examples

### 1. `embed_as_library.rs`

Shows how to use `nautpie`'s `ApiResponse` envelope directly, without going through the CLI. This is useful if you're writing a Rust program that wants to emit responses in the same JSON-line shape `nautpie` produces.

Run it with:

```sh
cargo run --example embed_as_library
```

**What it teaches:**

- How to construct an `ApiResponse::ok` and `ApiResponse::error`.
- The `to_line()` method that produces a single-line JSON string (matching the binary's stdout contract).

### 2. `build_deployment_payload.rs`

Builds a `DeploymentDetails` payload using the consuming builder pattern, then renders it to JSON. **No network calls** are made — this is purely about understanding the wire format.

Run it with:

```sh
cargo run --example build_deployment_payload
```

**What it teaches:**

- The fluent builder: `DeploymentDetails::new().ref_(...).ref_type(...).title(...)`.
- How `values()` strips null and empty fields (matching PHP's `array_filter($details, 'isNotNull')`).
- What the JSON payload actually looks like before it's POSTed to DeployNaut.

### 3. `parse_boolean_flag.rs`

Demonstrates the lenient boolean parser used by `nautpie` for flags like `--bypass_and_start`. The parser accepts `true`/`false`, `yes`/`no`, `1`/`0`, and treats missing or empty input as `false`.

Run it with:

```sh
cargo run --example parse_boolean_flag
```

**What it teaches:**

- The exact set of accepted truthy and falsy strings.
- That unknown values produce a typed `Error::Generic` rather than a panic.

---

## Walkthrough: `embed_as_library.rs`

```rust
use nautpie::error::ApiResponse;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = ApiResponse::ok(json!({"hello": "world"}));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.reason, "OK");
    ...
}
```

**Teaching points:**

- **`use nautpie::error::ApiResponse;`** — the example pulls in the public crate like any external user would. This proves the library is genuinely reusable.
- **`serde_json::json!({...})`** — the `json!` macro builds a `serde_json::Value` from JSON-like syntax. Cheaper than constructing a `Value::Object` by hand.
- **`Result<(), Box<dyn std::error::Error>>`** — the return type. `Box<dyn std::error::Error>` is the standard "any error" type for examples and small programs. The Azure guidelines recommend this for samples (`[rust-repo-samples-question-operator]`).
- **`println!("{line}");`** — single-line output with no trailing newline. `println!` adds the newline; `line` is already newline-free.

## Walkthrough: `build_deployment_payload.rs`

```rust
let details = DeploymentDetails::new()
    .ref_("abc123def456abc123def456abc123def4567890")
    .ref_type("sha")
    .title("[CD] v1.2.3")
    .summary("Branch:main")
    .bypass_and_start(true)
    .locked(false);

let json = details.values();
let pretty = serde_json::to_string_pretty(&json)?;
println!("{pretty}");
```

**Teaching points:**

- **Consuming builder.** Each method takes `mut self` and returns `Self`. This is a different style from the Azure-preferred `with_field()` pattern, but it works well for short-lived payload objects.
- **`serde_json::to_string_pretty(&json)?`** — render the `Value` as a multi-line pretty-printed JSON string. The `?` propagates any serialisation error (in practice, serialising a `Value` can never fail).
- **The full 40-character SHA.** The example uses a 40-character commit string because DeployNaut requires full SHAs in this position (see `do_create_tag` for the `len() != 40` check).

## Walkthrough: `parse_boolean_flag.rs`

```rust
for input in ["true", "TRUE", "yes", "YES", "1"] {
    let v = check_boolean(Some(input))?;
    assert!(v);
    println!("{input:?} -> {v}");
}
```

**Teaching points:**

- **`for input in [...]`** — iterate over a static array of `&str`. No allocation per iteration.
- **`assert!(v)`** — for booleans, prefer `assert!(v)` over `assert_eq!(v, true)`. Clippy's `bool_assert_comparison` lint enforces this.
- **`{:?}` for `Option<&str>`** — the `Debug` formatter prints `Some("yes")` or `None`, which is what we want here.

---

## Why Are Examples Useful?

- **Living documentation.** Unlike README examples (which can rot), `examples/` are compiled and run on every `cargo test`. If a public API breaks, the example fails to compile.
- **Discoverability.** A new contributor can `cargo run --example build_deployment_payload` and immediately see what the JSON payload looks like without reading the full library.
- **Reusable starting point.** Users embedding `nautpie` can copy an example as a starting template.
- **Tested doc snippets.** The Azure guidelines (`[rust-doc-samples]` and `[rust-repo-samples]`) recommend runnable examples in both doc comments (`///` blocks) and `examples/`. We do both.

---

## How Examples Are Wired In

`Cargo.toml` doesn't need any extra configuration to enable `examples/`. Cargo picks them up automatically:

```sh
$ cargo run --example embed_as_library
   Compiling nautpie v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.20s
    Running `target/debug/examples/embed_as_library`
{"status":200,"reason":"OK","body":{"hello":"world"}}
```

The compiled binary ends up in `target/debug/examples/<name>`. You can run it directly, or run it through `cargo run --example <name>`.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `examples/` directory | whole folder | Cargo treats each `.rs` file as a separate runnable program |
| `cargo run --example <name>` | invocation | Build and run a specific example |
| `use nautpie::...;` | imports | Use the library from outside the crate (proves the lib is reusable) |
| `fn main() -> Result<(), Box<dyn Error>>` | main signature | Required entry point; `Box<dyn Error>` is the standard "any error" type |
| `serde_json::json!(...)` | value construction | Build a `Value` from JSON-like syntax |
| `serde_json::to_string_pretty(&v)` | output | Pretty-print a `Value` as multi-line JSON |
| Consuming builder | `DeploymentDetails` | Each `with_*` returns `Self`, enabling `a.b().c().d()` chains |
| `assert!(v)` for booleans | `parse_boolean_flag` | Clippy enforces `assert!(v)` over `assert_eq!(v, true)` |
| `Box<dyn std::error::Error>` | return type | Type-erased error for small programs |
| `?` operator | error propagation | Early-return on `Err` |
