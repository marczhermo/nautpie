# `src/commands/mod.rs` — The Command Trait and Module Index

This file is the **module root** of the `commands/` folder. It does two things:

1. **Declares the submodules** (`sample`, `deploy_naut`, `bitbucket`) so they can be referenced as `nautpie::commands::sample`, etc.
2. **Defines the `Command` trait** that every concrete command implementation must satisfy.

The trait is the **heart of the dispatch system**. In `main.rs`, you saw this:

```rust
let cmd = DeployNaut;
cmd.run(&normalise_action(&action_raw), io, http)
```

That `.run()` call works because `DeployNaut` implements the `Command` trait defined here.

---

## Walkthrough

### Module declarations

```rust
pub mod bitbucket;
pub mod deploy_naut;
pub mod sample;
```

Three submodules of `commands/`. Each one corresponds to a Rust file:

- `bitbucket.rs` → `ci:bitbucket` actions
- `deploy_naut.rs` → `deploy:naut` actions
- `sample.rs` → `sampleSuccess` / `sampleFail` smoke-test actions

These declarations are required so the modules can be referenced through `commands::sample`, etc. Without them, even though the files exist on disk, Rust wouldn't know they were part of the crate.

### The `Command` trait

```rust
use crate::error::{ApiResponse, Error};
use crate::http::HttpClient;
use crate::io::Io;

#[allow(dead_code)]
pub trait Command {
    fn name(&self) -> &'static str;

    fn run(
        &self,
        action: &str,
        io: &mut dyn Io,
        http: &mut dyn HttpClient,
    ) -> Result<ApiResponse, Error>;
}
```

**Teaching points:**

- **`pub trait Command`** — defines a new trait. A trait is like an **interface** in other languages: it lists methods that any implementing type must provide. Concrete types say "I implement Command" with `impl Command for Foo { ... }`.
- **`fn name(&self) -> &'static str`** — every command must have a name (e.g., `"deploy:naut"` or `"ci:bitbucket"`). `&'static str` is a string slice with `'static` lifetime, meaning the string lives for the entire duration of the program — usually a string literal.
- **`fn run(&self, action: &str, io: &mut dyn Io, http: &mut dyn HttpClient) -> Result<ApiResponse, Error>`** — the main entry point. Three things to notice:
  - `&self` — borrowed self. The trait object doesn't own the data.
  - `&mut dyn Io`, `&mut dyn HttpClient` — **trait objects**. These can be any type implementing the trait. The `dyn` keyword is short for "dynamic dispatch" — at runtime, Rust looks up which concrete method to call through a vtable. This is how tests can substitute a fake `Io` or `HttpClient`.
  - `Result<ApiResponse, Error>` — returns either the successful response envelope or an error.
- **`#[allow(dead_code)]`** — suppresses the warning that `name()` is currently never called. The trait defines it for future use, but the dispatcher in `main.rs` doesn't actually call it.

### What the implementors look like

In `deploy_naut.rs`:

```rust
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
            // ... etc ...
            _ => Err(Error::MissingAction),
        }
    }
}
```

The `DeployNaut` struct is **a unit struct** (no fields). It exists only as a "tag" — a way to attach the `Command` impl to a concrete type. There's only ever one value of it: `DeployNaut`.

`Bitbucket` in `bitbucket.rs` works the same way.

### Why a trait?

Without the `Command` trait, the dispatcher in `main.rs` would have to know about every concrete command type and `match` on them:

```rust
match command_name {
    "deploy:naut" => deploy_naut::DeployNaut.run(action, io, http),
    "ci:bitbucket" => bitbucket::Bitbucket.run(action, io, http),
    // ... etc ...
}
```

This is fine for two commands but gets unwieldy at ten. With the trait, you can store a `&dyn Command` and call `.run()` on it uniformly. The trait also enables generic code: `fn run_any(cmd: &impl Command, ...) { cmd.run(...); }`.

In this codebase the dispatch is small enough that the trait's main value is **discoverability** — it tells you "here are the two methods every command must implement."

---

## Why Is It Written This Way?

- **One trait, many implementations.** This is the textbook use of a Rust trait. It defines a contract, and the concrete types (`DeployNaut`, `Bitbucket`) fulfill it.
- **`&mut dyn Io` and `&mut dyn HttpClient`.** Trait objects (`dyn Trait`) are how Rust does **runtime polymorphism**. They have a small cost (vtable lookup on every call) but enable dependency injection — perfect for tests.
- **`#allow(dead_code)]` on the trait.** The `name()` method isn't currently called, but defining it documents the contract and gives us a place to hang metadata later (e.g., help text per subcommand).
- **The `_ => Err(Error::MissingAction)` arm.** Every `match` in Rust must be **exhaustive**: every possible input value must be covered. The catch-all `_` ensures that an unknown action name produces a clean error rather than a panic.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub mod name;` | top of file | Declares a submodule wired to `commands/name.rs` |
| `pub trait Name` | `Command` | Defines a trait — a contract of methods |
| `&self` | trait methods | Borrowed self |
| `&mut dyn Trait` | `run` params | Mutable reference to a trait object |
| `Result<T, E>` | `run` return | Success-or-failure return type |
| `&'static str` | `name` return | String slice with `'static` lifetime |
| `impl Trait for Type` | implementors | Implement a trait for a concrete type |
| Unit struct `pub struct Name;` | `DeployNaut` | A struct with no fields — just a tag |
| `#[allow(dead_code)]` | trait | Suppress unused-code warnings |
| Catch-all `_` | `match` arms | Match any value not covered above |
