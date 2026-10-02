# `src/tui/mod.rs` — The TUI Module Index

This file is the **module root** of the `tui/` folder. It's tiny — just two lines — but it teaches a common Rust pattern: a module that's mostly there to **organize related code**.

---

## The Whole File

```rust
//! Terminal progress UI used by `gitFetch` and `checkDeploymentProgress`.
//!
//! In a TTY, a ratatui spinner renders the elapsed time and the most recent
//! status from the polled HTTP endpoint. In a non-TTY (CI), the spinner
//! falls back to a stderr "Waiting…" line every 5 seconds, matching PHP's
//! behaviour.

pub mod spinner;

pub use spinner::{run_with_progress, PollOutcome, ProgressRunner};
```

---

## Walkthrough

### The doc comment

```rust
//! Terminal progress UI used by `gitFetch` and `checkDeploymentProgress`.
//!
//! In a TTY, a ratatui spinner renders the elapsed time and the most recent
//! status from the polled HTTP endpoint. In a non-TTY (CI), the spinner
//! falls back to a stderr "Waiting…" line every 5 seconds, matching PHP's
//! behaviour.
```

`//!` is an **inner doc comment** — it documents the module itself, not an item inside it. `cargo doc` will render this as the documentation page for the `tui` module.

The comment explains the **two-mode design**: TTY (interactive terminal) and non-TTY (CI). The same code path produces different output depending on the environment.

### Module declaration

```rust
pub mod spinner;
```

Declares the `spinner` submodule, which lives in `spinner.rs` next to this file. The `spinner` module contains all the implementation.

### Re-exports

```rust
pub use spinner::{run_with_progress, PollOutcome, ProgressRunner};
```

This is the same re-export pattern we saw in `lib.rs` and `http/mod.rs`. Three items from `spinner` are re-exported at the `tui::` level:

- **`run_with_progress`** — the convenience function that builds a runner and runs it.
- **`PollOutcome`** — the enum returned by the polling callback.
- **`ProgressRunner`** — the struct that holds configuration and runs the polling loop.

So callers can write:

```rust
use nautpie::tui::{run_with_progress, PollOutcome};
```

instead of the longer:

```rust
use nautpie::tui::spinner::{run_with_progress, PollOutcome};
```

---

## Why Is It Written This Way?

- **Module as namespace.** Even though there's only one submodule, having a `tui` namespace keeps the public API organized. If we ever add more terminal UI features (a progress bar, an interactive prompt), they'd go here too.
- **Re-exports for ergonomics.** The most commonly-used items from `spinner` are re-exported at the top level. Callers don't need to know the internal submodule structure.
- **Documentation at the module level.** The doc comment tells you the **why** of the module — the dual TTY/non-TTY design — before you dive into the implementation.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `//!` | doc comment | Inner doc comment — documents the enclosing module |
| `pub mod spinner;` | top | Declare a submodule |
| `pub use path::{Items...};` | bottom | Re-export items at a shorter path |
| Brace import `{a, b, c}` | bottom | Import multiple items from one path |
