# `src/io/stderr_io.rs` — The Real Output Sink

This file implements the production `Io` trait. It writes to stderr (for human messages) and stdout (for the JSON envelope). Coloured badges make the warnings and successes visually distinct, just like the PHP original.

---

## The Big Picture

Every command invocation produces output on two streams:

- **stdout**: the final JSON-line response (always exactly one line).
- **stderr**: status messages (warnings, success ticks, info, debug logs from the HTTP client).

`StderrIo` is the concrete type that does this writing. It also stores the parsed CLI options so action functions can pull them out later.

---

## Walkthrough

### Imports

```rust
use std::io::{self, Write};

use owo_colors::{OwoColorize, Stream};

use super::Io;
use crate::error::ApiResponse;
use crate::options::Options;
```

- **`std::io::{self, Write}`** — `io` brings in the `std::io` module (we use `io::stderr()` and `io::stderr().lock()`). `Write` is the trait that provides `.writeln!(...)`. The `self` in `io::{self, Write}` lets us refer to the module as `io` rather than `std::io`.
- **`owo_colors::{OwoColorize, Stream}`** — `OwoColorize` is the trait that gives us `.if_supports_color(...)`. `Stream::Stderr` is the stream we're targeting (we want colours only on stderr, not stdout).

### The struct

```rust
pub struct StderrIo {
    opts: Option<Options>,
}
```

A tiny struct with one field:

- **`opts: Option<Options>`** — the parsed CLI options, stashed by `main.rs` via `set_options` and read back by action functions via `options`. Wrapped in `Option` because it might not have been set yet.

Note the field is **not** `pub` — it's private. The trait methods are the only way to access it.

### `new()` and `Default`

```rust
impl StderrIo {
    pub fn new() -> Self {
        Self { opts: None }
    }
}

impl Default for StderrIo {
    fn default() -> Self {
        Self::new()
    }
}
```

A trivial constructor and a `Default` impl that delegates to `new()`. The `Default` impl is auto-implementable in many cases, but here we write it explicitly to delegate to `new()`.

**Teaching points:**

- **`Self { opts: None }`** — struct literal with one field set to `None`. Equivalent to `Self { opts: None }`.
- **`impl Default for StderrIo`** — implements the `Default` trait, so `StderrIo::default()` works.

### The `Io` impl

```rust
impl Io for StderrIo {
    fn warning(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "  {}  {}  ",
            "WARN".if_supports_color(Stream::Stderr, |t| t.on_yellow().red()),
            message
        );
    }

    fn success(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "  {}  {}  ",
            " OK ".if_supports_color(Stream::Stderr, |t| t.on_green().black()),
            message
        );
    }

    fn message(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(stderr, "  {}  ", message);
    }

    fn write_json(&mut self, response: &ApiResponse) {
        println!("{}", response.to_line());
    }

    fn set_options(&mut self, opts: Options) {
        self.opts = Some(opts);
    }

    fn options(&self) -> Option<Options> {
        self.opts.clone()
    }
}
```

#### `warning()` — yellow-on-red badge

```rust
fn warning(&mut self, message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(
        stderr,
        "  {}  {}  ",
        "WARN".if_supports_color(Stream::Stderr, |t| t.on_yellow().red()),
        message
    );
}
```

**Teaching points:**

- **`io::stderr()`** — get a handle to the standard error stream.
- **`.lock()`** — acquire an exclusive lock on the stream. This ensures that if multiple threads wrote to stderr simultaneously, their output wouldn't interleave. In this single-threaded CLI it doesn't matter much, but it's a good habit.
- **`writeln!(stderr, "...")`** — write a formatted line (with a trailing newline). Returns `Result<(), io::Error>`. We use `writeln!` rather than `println!` because we're writing to a specific handle (`stderr`), not the default stdout.
- **`let _ = writeln!(...)`** — discard the `Result`. We don't care about write errors (there's no useful recovery for stderr failures).
- **`"WARN".if_supports_color(Stream::Stderr, |t| t.on_yellow().red())`** — this is where the magic happens.
  - `"WARN"` is a `&str`. The `OwoColorize` trait provides `.if_supports_color(...)`.
  - **`Stream::Stderr`** — only apply colours if stderr supports them (i.e., is a TTY). When stderr is piped to a file or another program, no colour codes are emitted.
  - **`|t| t.on_yellow().red()`** — the closure receives the coloured text and applies styling: yellow background, red foreground. The result is the colour-coded ANSI escape sequence string.

The output looks like:

```
  WARN  Some warning message  
```

With the `WARN` text in red on a yellow background, surrounded by spaces.

#### `success()` — black-on-green badge

Same pattern as `warning`, but with `on_green().black()`. The output:

```
  OK   Some success message
```

#### `message()` — neutral info line

```rust
fn message(&mut self, message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "  {}  ", message);
}
```

No colour. Just prints the message in a `  message  ` format. Used for neutral informational output.

#### `write_json()` — the single stdout line

```rust
fn write_json(&mut self, response: &ApiResponse) {
    println!("{}", response.to_line());
}
```

This is the **only** method that writes to stdout. It uses `println!` (which writes to stdout by default) rather than `writeln!(stdout, ...)`. The `to_line()` method on `ApiResponse` produces a single line of JSON without a trailing newline; `println!` adds the newline.

This method is the keystone of the program's contract: **exactly one JSON line on stdout, always**.

#### `set_options()` and `options()`

```rust
fn set_options(&mut self, opts: Options) {
    self.opts = Some(opts);
}

fn options(&self) -> Option<Options> {
    self.opts.clone()
}
```

`set_options` stores the bag. `options` returns a clone (because `Options` derives `Clone` and the trait signature requires `Option<Options>`).

The `.clone()` is necessary because returning a reference would tie the returned value to `self`'s lifetime, which is awkward. Since `Options` is just a thin wrapper around a `HashMap<String, String>`, cloning is cheap.

---

## Why Is It Written This Way?

- **PHP parity.** The original PHP used `<fg=red;bg=yellow>` and `<fg=black;bg=green>` Symfony tags for the warning and success badges. The Rust port uses ANSI escape codes (via `owo-colors`) to produce the same visual effect.
- **TTY-aware colouring.** `if_supports_color(Stream::Stderr, ...)` automatically disables colour when stderr is piped or redirected. This means CI logs don't get cluttered with escape codes.
- **stderr/stdout separation.** Status messages go to stderr; the JSON envelope goes to stdout. CI scripts can pipe `nautpie | jq` and get clean JSON without status messages polluting the stream.
- **Locked stderr.** Even though this CLI is single-threaded, locking the handle is a small habit that prevents bugs if the code is ever extended to use threads.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `pub struct StderrIo { opts: Option<Options> }` | struct | Data struct with one private field |
| `Self { opts: None }` | `new` | Struct literal syntax |
| `impl Io for StderrIo` | top | Implement the `Io` trait |
| `io::stderr()` | methods | Get the stderr handle |
| `.lock()` | methods | Acquire an exclusive lock on the handle |
| `writeln!(handle, "...")` | methods | Write a formatted line with trailing newline |
| `let _ = writeln!(...)` | methods | Discard the `Result` from `writeln!` |
| `println!("...")` | `write_json` | Write to stdout |
| `owo-colors::OwoColorize` | coloured methods | Trait that adds colour methods to strings |
| `.if_supports_color(stream, \|t\| ...)` | coloured methods | Apply colour only if the stream supports it |
| Closure `\|t\| t.on_yellow().red()` | coloured methods | Inline function — apply colour attributes |
| `Stream::Stderr` | coloured methods | Enum variant identifying stderr |
| `impl Default for StderrIo` | bottom | Implement the `Default` trait |
| `Option<Options>` | `opts` field | "Some options, or no options" |
| `.clone()` | `options` | Duplicate an `Options` value |
| `_opts` parameter | unused param | Underscore prefix = unused parameter |
| `&mut self` | mutating methods | Method may modify state |
| `&self` | `options` | Method only reads |
