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
//!
//! # Crate overview
//!
//! ... module map table ...
//!
//! # Quickstart
//!
//! ```no_run
//! use nautpie::error::ApiResponse;
//! use serde_json::json;
//! let resp = ApiResponse::ok(json!("hello"));
//! ```

#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

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

That's the entire file. Every single line is doing important work.

---

## Walkthrough

### The doc comment block

```rust
//! NautPie — DeployNaut / Bitbucket Pipelines deployment client.
//! ...
//! # Module map
//! ...
//! # Quickstart
//! ```no_run
//! ...
//! ```
```

`//!` is an **inner doc comment** — it documents the **module itself**, not an item inside it. `cargo doc` will render this as the documentation page for the `nautpie` crate.

This particular comment block has grown over time. It now includes:

- **A one-line description** of what the crate does.
- **A "Crate overview"** section that points at the README.
- **A "Module map" table** listing every public module with a one-line summary.
- **A "Quickstart" section** with a `no_run` Rust code example showing the most basic usage (building an `ApiResponse`).

The `no_run` attribute on the code fence is important: it tells `cargo test` to **compile** the example (so it can't go stale) but **not run** it (so it doesn't require any external resources).

### Crate-level attributes

```rust
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
```

**Teaching points:**

- **`#![...]` is an inner attribute** — the `!` means it applies to the enclosing item (the crate), not to a single item below. It always goes at the top of the file (or near the top).
- **`#![warn(missing_docs)]`** — turns on the `missing_docs` lint as a **warning**. This means every `pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub mod`, and public field without a `///` doc comment will produce a compiler warning. This is one of the recommendations from the Azure SDK Rust guidelines (item `[rust-documentation-warn-missing-docs]`) and a huge help in keeping documentation complete.
- **`#![doc = include_str!("../README.md")]`** — the `include_str!` macro reads a file at compile time and embeds its contents as a `&'static str`. The `#![doc = ...]` attribute then treats that string as **module-level documentation**. The result: `cargo doc --open` shows the README as the crate overview, exactly the same way it appears on GitHub.
  - The path `../README.md` is **relative to the file containing the attribute** (`src/lib.rs`), so `../README.md` means "go up one directory to the crate root, then into `README.md`."

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
  2. **Reusability.** If someone wants to call `nautpie` as a library function rather than spawning the binary, they can. The `examples/` directory in the crate root demonstrates this.
- **Why re-export `ApiResponse` and `Error`?** These are the two types a caller is most likely to want. Putting them at the crate root means shorter `use` statements in client code.
- **`#![warn(missing_docs)]`** — this is the single most important documentation improvement from the Azure SDK guidelines review. Without it, public items can quietly go undocumented. With it, every future PR that adds an undocumented `pub fn` will produce a compiler warning.
- **`#![doc = include_str!("../README.md")]`** — keeps `cargo doc` in sync with GitHub. Without it, the rendered documentation has a thin placeholder intro; with it, callers see the full README including examples and troubleshooting.
- **Doc comments (`//!`).** The triple-slash comments at the top are **inner doc comments** — they document the **module itself**, not an item inside it.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `//!` | doc comment | Inner doc comment — documents the enclosing module |
| `pub mod name;` | module decls | Declares a public module wired up to `src/name.rs` or `src/name/mod.rs` |
| `pub use path::Item;` | re-export | Re-exports an item so callers can use a shorter path |
| `lib.rs` vs `main.rs` | entry points | The library and binary entry points of a Cargo project |
| `#![warn(lint)]` | crate attribute | Promote a lint to a warning for the whole crate |
| `#![doc = "..."]` | crate attribute | Set the crate-level doc comment programmatically |
| `include_str!("path")` | macro | Embed a file's contents as a `&'static str` at compile time |
| `\`\`\`no_run` | doc fence | Code compiles but doesn't run as part of `cargo test` |
| `\`\`\`rust no_run` | doc fence | Same, explicitly tagged as Rust |

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
