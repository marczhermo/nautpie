# `src/lib.rs` — The Crate's Public Surface

When `Cargo.toml` lists both a `[[bin]]` and a `[lib]` section, you end up with **two crates from one project**: a binary (the `nautpie` executable) and a library (a reusable code module other Rust programs can depend on). `src/lib.rs` is the entry point of that library.

In this project, the binary is tiny — `main.rs` is only about 70 lines. The library is where the real work lives. Splitting them like this lets the integration tests (`tests/*.rs`) use the library code directly, and it would let other Rust programs embed `nautpie` as a dependency if they ever wanted to.

---

## The Whole File

```rust
//! NautPie — DeployNaut / Bitbucket Pipelines deployment client.
//!
//! Rust port of the PHP `nautpie.phar` (Symfony Console). This crate exposes the
//! CLI dispatch surface and supporting helpers; the binary entrypoint is
//! [`main`] in `src/main.rs`.

pub mod boolean;
pub mod cli;
pub mod commands;
pub mod deployment_details;
pub mod env;
pub mod error;
pub mod http;
pub mod io;
pub mod options;
pub mod tui;

pub use error::{ApiResponse, Error};
```

That's the entire file. Eight lines of declarations. But every single line is doing important work.

---

## Walkthrough

### Module declarations

```rust
pub mod boolean;
pub mod cli;
pub mod commands;
pub mod deployment_details;
pub mod env;
pub mod error;
pub mod http;
pub mod io;
pub mod options;
pub mod tui;
```

- `pub mod` means "this is a **public module** of the library." Anyone depending on this crate can write `nautpie::cli::Cli` and reach into the `cli` module.
- Each `mod` name corresponds to a file or folder in `src/`. For example, `pub mod http;` resolves to either `src/http.rs` or `src/http/mod.rs`. Here it's the folder form (the `http/mod.rs` we read earlier).
- The ordering is alphabetical, which is a common Rust convention.

Why do we need to declare modules here? In Rust, **modules are private by default**. Declaring them in `lib.rs` (or `main.rs`) brings them into scope. Declaring them as `pub` exposes them to outside consumers of the library.

### Re-exports

```rust
pub use error::{ApiResponse, Error};
```

This is a **re-export**. Without it, consumers would have to write the full path:

```rust
use nautpie::error::{ApiResponse, Error};
```

With the re-export, they can write the shorter:

```rust
use nautpie::{ApiResponse, Error};
```

It's purely a convenience — it makes the most commonly-used types from the library available at the crate's top level. The original definitions are still in `src/error.rs`; the re-export just adds a shortcut.

---

## The Module Tree

This single file creates the entire module hierarchy that the rest of the codebase uses. Here's the tree:

```
nautpie                  (the crate root — this file)
├── boolean              →  src/boolean.rs
├── cli                  →  src/cli.rs
├── commands             →  src/commands/mod.rs
│   ├── bitbucket        →  src/commands/bitbucket.rs
│   ├── deploy_naut      →  src/commands/deploy_naut.rs
│   └── sample           →  src/commands/sample.rs
├── deployment_details   →  src/deployment_details.rs
├── env                  →  src/env.rs
├── error                →  src/error.rs            (ApiResponse, Error)
├── http                 →  src/http/mod.rs
│   └── reqwest_client   →  src/http/reqwest_client.rs
├── io                   →  src/io/mod.rs
│   └── stderr_io        →  src/io/stderr_io.rs
├── options              →  src/options.rs
└── tui                  →  src/tui/mod.rs
    └── spinner          →  src/tui/spinner.rs
```

That's why `main.rs` can write things like `use nautpie::cli::Cli;` or `use nautpie::error::{ApiResponse, Error};` — every piece is wired up here.

---

## Why Is It Written This Way?

- **Binary + library split.** Rust makes it trivial to have both at once: declare a binary in `Cargo.toml` with `[[bin]]` and a library with `[lib]`. The library has its own entry point (`src/lib.rs`); the binary has its own (`src/main.rs`). They share the source tree but produce two separate compilation artifacts.
- **Why not just put everything in `main.rs`?** Two big reasons:
  1. **Integration tests** in `tests/*.rs` need to `use nautpie::something;` — they can only do that if the code is in the library, not the binary.
  2. **Reusability.** If someone wants to call `nautpie` as a library function rather than spawning the binary, they can.
- **Why re-export `ApiResponse` and `Error`?** These are the two types a caller is most likely to want. Putting them at the crate root means shorter `use` statements in client code.
- **Doc comments (`//!`).** The triple-slash comments at the top are **inner doc comments** — they document the **module itself**, not an item inside it. `cargo doc` will render them as the documentation page for the `nautpie` crate.

---

## Key Rust Concepts Used Here

| Concept | What it does |
|---|---|
| `//!` | Inner doc comment — documents the enclosing module |
| `pub mod name;` | Declares a public module wired up to `src/name.rs` or `src/name/mod.rs` |
| `pub use path::Item;` | Re-exports an item so callers can use a shorter path |
| `lib.rs` vs `main.rs` | The library and binary entry points of a Cargo project |

---

## How `main.rs` Uses This

Every `use` statement at the top of `main.rs` resolves through this file:

```rust
use nautpie::cli::{normalise_action, Cli, CommandKind};
use nautpie::commands::bitbucket::Bitbucket;
use nautpie::commands::deploy_naut::{
    options_from_bitbucket_args, options_from_deploy_args, DeployNaut,
};
use nautpie::commands::Command;
use nautpie::env::load_dotenv;
use nautpie::error::{ApiResponse, Error};
use nautpie::http::{HttpClient, ReqwestHttpClient};
use nautpie::io::{Io, StderrIo};
```

Every one of these paths starts from the crate root defined right here.
