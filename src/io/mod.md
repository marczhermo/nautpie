# `src/io/mod.rs` — The Output Sink Abstraction

This file defines the **interface** for the binary's output. Just like `http/mod.rs` defines how the program *sends*, this file defines how the program *speaks*. The `Io` trait abstracts over the destination: in production, it writes to stderr and stdout; in tests, it can record messages for later inspection.

---

## The Big Picture

The CLI's contract with the outside world is:

- **stdout**: exactly one JSON line, always.
- **stderr**: human-readable status messages (warnings, success ticks, info).

The `Io` trait models this contract. Every action function takes `&mut dyn Io` and calls methods like `warning`, `success`, and `message` (for stderr) and `write_json` (for stdout).

This file defines:

1. **`Io` trait** — the interface.
2. **`StderrIo` re-export** — the production impl, defined in `stderr_io.rs`.
3. **`Options` re-export** — used by the `set_options` / `options` methods on `Io`.

---

## Walkthrough

### Imports and re-exports

```rust
use crate::error::ApiResponse;
use crate::options::Options;

pub mod stderr_io;
pub use stderr_io::StderrIo;
```

- **`pub mod stderr_io;`** — declares the submodule containing the production impl.
- **`pub use stderr_io::StderrIo;`** — re-exports at the `io::` level. Callers can write `use nautpie::io::StderrIo;` instead of `use nautpie::io::stderr_io::StderrIo;`.

### The `Io` trait

```rust
pub trait Io {
    fn warning(&mut self, message: &str);
    fn success(&mut self, message: &str);
    fn message(&mut self, message: &str);
    fn write_json(&mut self, response: &ApiResponse);

    fn set_options(&mut self, _opts: Options) {}
    fn options(&self) -> Option<Options> { None }
}
```

**Teaching points:**

- **`pub trait Io`** — defines the interface. Concrete types (`StderrIo` in production, mocks in tests) implement it.
- **`&mut self`** on `warning`, `success`, `message`, `write_json`, `set_options` — these methods may modify state (e.g., write to a buffer). Mutable borrow required.
- **`&self`** on `options` — only reads. Immutable borrow.
- **`fn warning(&mut self, message: &str)`** — print a yellow-on-red warning badge. The default behaviour is impl-specific.
- **`fn success(&mut self, message: &str)`** — print a black-on-green success badge.
- **`fn message(&mut self, message: &str)`** — print a neutral info line.
- **`fn write_json(&mut self, response: &ApiResponse)`** — write the final JSON-line response to stdout. Note: this is the **only** method that goes to stdout, by design.
- **`fn set_options(&mut self, _opts: Options)`** — inject the parsed CLI options bag. The **default implementation** is `{}` (a no-op). This is one of Rust's most useful features: **default methods on traits**. If an implementor doesn't need options (a test mock, for example), they get a no-op for free.
- **`fn options(&self) -> Option<Options> { None }`** — retrieve the injected options. The default is `None`. Real implementors (like `StderrIo`) override this.
- **`_opts: Options`** — the underscore prefix on `_opts` is significant here. In the default implementation, the parameter is **named with a leading underscore** so the compiler doesn't warn about it being unused. This is a convention for "this parameter exists for the trait contract, but the default impl doesn't need it."

### Why have a no-op default for `set_options` and `options`?

Without default implementations, every `Io` implementor would be forced to write:

```rust
fn set_options(&mut self, _opts: Options) {}
fn options(&self) -> Option<Options> { None }
```

...even if they don't care about options. With the defaults, only the production impl (`StderrIo`) overrides them. Test mocks can ignore options entirely.

This is a key insight about Rust traits: they're not just contracts, they're **partial implementations**. You can write as much shared logic in the trait itself as you like, and let implementors override only what they need to.

### `Options` and the side-channel pattern

Notice that `Io` carries an `Options` bag. This is unusual: why does the output sink hold input data?

The answer is **trait signature constraints**. The `Command::run` method takes `&mut dyn Io, &mut dyn HttpClient` — those are the only two parameters it gets. There's no slot for "the original CLI args."

So `main.rs` stuffs the parsed options into the `Io` impl via `set_options`, and the action functions read them back via `io.options()`. It's a **side-channel** that lets the action functions access data that wouldn't otherwise fit through the trait signature.

This pattern shows up in many places in Rust: when a trait is too narrow to carry all the data you need, you attach the data to one of the trait objects. It's a bit of a hack, but it's pragmatic.

---

## Why Is It Written This Way?

- **PHP parity through abstraction.** The PHP original had a `SymfonyStyle` output wrapper with methods like `warning`, `success`, etc. The Rust port models this as a trait, which lets tests substitute mocks.
- **Default methods.** Putting `set_options` and `options` defaults in the trait means simple implementors don't have to write no-op stubs.
- **Side-channel via `Io`.** A pragmatic solution to the trait signature constraint. It works because the options are read once and discarded.
- **One trait, two consumers.** The production code uses `StderrIo` (real output). The tests use mocks that record what was written. Both implement the same trait.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub mod name;` | top | Declare a submodule |
| `pub use path::Item;` | top | Re-export at a shorter path |
| `pub trait Io` | trait | Define an interface |
| `&mut self` | methods | Mutable borrow — method may modify state |
| `&self` | `options` | Immutable borrow — method only reads |
| `&str` parameter | string args | Borrowed string slice |
| Default method body `{}` | `set_options` | "This method does nothing by default" |
| Default method body `None` | `options` | "This method returns `None` by default" |
| `_opts` | parameter | Underscore prefix = unused parameter |
| Side-channel | `set_options` / `options` | Stash extra data through one of the trait objects |
| `Option<Options>` | `options` return | "Some options, or no options" |
