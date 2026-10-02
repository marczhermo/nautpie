# `src/main.rs` — The Program Entry Point

This file is the **front door** of the `nautpie` binary. When you run `nautpie deploy:naut createDeployment`, the code inside `main()` is what gets executed first.

Think of `main.rs` as the **conductor of an orchestra**: it does not play any instruments itself (it does not actually talk to APIs), but it tells every other piece when to start, what to do, and how to report back.

---

## The Big Picture

The whole program boils down to five steps:

1. Load environment variables from a `.env` file (if one exists).
2. Parse the command-line arguments using `clap`.
3. Set up the "output sink" (stderr) and the "HTTP client" (reqwest).
4. Dispatch to the right subcommand (`deploy:naut` or `ci:bitbucket`) with the right action.
5. Print a single JSON line on stdout, then exit with code `0` (success) or `1` (failure).

That's the **contract** with the outside world: one JSON line on stdout, an exit code, and that's it. Everything else (status messages, warnings, progress bars) goes to stderr so it doesn't pollute the JSON the caller is parsing.

---

## Walkthrough

### The `main()` function

```rust
fn main() {
    let _ = load_dotenv();

    let cli = Cli::parse();
    let mut io = StderrIo::new();
    let mut http = ReqwestHttpClient::new().expect("reqwest client build");

    let exit_code = dispatch(cli, &mut io, &mut http);
    std::process::exit(exit_code);
}
```

Every Rust program starts at a function called `main`. It takes no arguments and returns nothing (well, technically it returns `()` — Rust's name for "nothing"). Anything you want to return to the operating system must be passed to `std::process::exit()`.

**A few teaching points:**

- `let _ = load_dotenv();` — The underscore prefix `_` tells the compiler: "I know this returns a value, but I deliberately don't want to use it." Without that underscore, the compiler would warn you about an unused result. The `Result` returned by `load_dotenv()` is silently discarded because loading `.env` is **best-effort**: if there's no `.env` file, that's fine, we just use real environment variables.
- `Cli::parse()` — `Cli` is a struct defined in `src/cli.rs`. The `#[derive(Parser)]` attribute from the `clap` crate generates all the argument-parsing code for you at compile time. `parse()` looks at `std::env::args()` (the actual `argv` from the shell) and produces a fully-populated `Cli` struct.
- `let mut io = StderrIo::new();` — The `mut` keyword means **mutable**. By default, Rust variables are immutable (read-only). `io` needs to be mutable because we'll be calling methods like `set_options` on it that change its internal state.
- `.expect("reqwest client build")` — `ReqwestHttpClient::new()` returns a `Result`. If it's an `Err`, `expect` will **panic** (crash the program) with the given message. We use it here because if we can't even build an HTTP client, there's no point continuing. Azure SDK guidelines discourage `expect`, but a process-level init that can never realistically fail is an acceptable exception.
- `std::process::exit(exit_code)` — terminates the program immediately with the given exit code. Unix convention: `0` means success, anything else means failure.

---

### The `dispatch()` function

```rust
fn dispatch(cli: Cli, io: &mut dyn Io, http: &mut dyn HttpClient) -> i32 {
    let action_raw = match &cli.command {
        CommandKind::DeployNaut(args) => args.action.clone(),
        CommandKind::Bitbucket(args) => args.action.clone(),
    };

    match &cli.command {
        CommandKind::DeployNaut(args) => io.set_options(options_from_deploy_args(args)),
        CommandKind::Bitbucket(args) => io.set_options(options_from_bitbucket_args(args)),
    }

    let result = match cli.command {
        CommandKind::DeployNaut(_) => {
            let cmd = DeployNaut;
            cmd.run(&normalise_action(&action_raw), io, http)
        }
        CommandKind::Bitbucket(_) => {
            let cmd = Bitbucket;
            cmd.run(&normalise_action(&action_raw), io, http)
        }
    };

    finish(result, io)
}
```

**Teaching points:**

- **`fn dispatch(cli: Cli, io: &mut dyn Io, http: &mut dyn HttpClient) -> i32`** — Three parameters, returns an `i32` (the exit code). The `&mut dyn Io` and `&mut dyn HttpClient` types are interesting:
  - `&mut` means "a mutable reference." We're lending the function the right to modify `io` and `http`, but ownership stays with `main()`.
  - `dyn Io` means "any type that implements the `Io` trait." This is **dynamic dispatch** — Rust will look up which method to call at runtime through a vtable. We use this so tests can swap in a fake `Io` that records what was written.
- **The first `match`** extracts the raw action string (`sampleSuccess`, `createAccessToken`, etc.) by matching on the `CommandKind` enum.
- **The second `match`** puts the parsed CLI options onto the `io` object. This is a bit unusual — it's stashing data on the `io` object so the action functions can later pull it back out with `io.options()`. Why? Because the `Command` trait signature only takes `&mut dyn Io` and `&mut dyn HttpClient`, not the original CLI args. The `Io` trait acts as a **side-channel** for passing options.
- **`cli.command` is moved twice?** — No, Rust is smart about this. The first two matches use `&cli.command` (borrow), and only the third one moves it. That's why the first two don't have `mut`.
- **`&normalise_action(&action_raw)`** — `normalise_action` is a helper that converts any case style (`sampleSuccess`, `SAMPLESUCCESS`, `sample_success`, `Sample-Success`) into the canonical `lowerCamelCase` form. The `&` means we're passing a borrow, so the function can read the string without taking ownership.

---

### The `finish()` function

```rust
fn finish(result: Result<ApiResponse, Error>, io: &mut dyn Io) -> i32 {
    match result {
        Ok(response) => {
            io.write_json(&response);
            0
        }
        Err(err) => {
            io.warning(&err.to_string());
            let body = serde_json::Value::String(err.to_string());
            let response = ApiResponse::error(1, "Bad Request", body);
            io.write_json(&response);
            1
        }
    }
}
```

**Teaching points:**

- **`Result<ApiResponse, Error>`** — Rust's built-in enum for "either success or failure." It has two variants: `Ok(value)` for success and `Err(error)` for failure. There's no exceptions in Rust — errors are values that you pass around.
- **`io.write_json(&response)`** — `write_json` is defined on the `Io` trait. Because we passed in `&mut dyn Io`, Rust calls the right implementation at runtime.
- **The error branch** does two things: prints a human-readable warning on stderr, then **also** prints a JSON line on stdout with status `1` and reason `"Bad Request"`. This is for **CI scripts that parse stdout**: they always get exactly one JSON line, whether things went well or badly. The warning on stderr is a bonus for humans watching the terminal.
- **The implicit return** — note the function has no `return` keyword and no semicolon on the last expression of each match arm. In Rust, the last expression in a block (without a semicolon) is the value of that block. So `0` and `1` are returned implicitly.

---

## Why Is It Written This Way?

- **One JSON line, always.** The whole binary is designed so that a CI script can pipe stdout to `jq` and get a clean structured answer. That's why all progress messages go to stderr.
- **Trait objects (`dyn Io`, `dyn HttpClient`).** These let tests substitute fake implementations. The integration tests in `tests/` rely on this heavily.
- **A thin `main.rs`.** All the real logic lives in libraries (`src/lib.rs` and modules). This makes the code easier to test (you don't need to spawn a real process) and easier to embed in another program if you ever want to.
- **`finish()` is its own function.** It exists so the dispatcher has a single, clean exit point — every path through the program ends by calling `finish`, which handles both success and failure uniformly.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `fn main()` | top of file | Required entry point of every Rust binary |
| `let mut` | in `main()` | Declares a mutable variable |
| `let _ = ...` | `load_dotenv()` | Discards a `Result` you intentionally don't use |
| `Result<T, E>` | everywhere | Rust's success-or-error type |
| `match` | `dispatch`, `finish` | Pattern matching — like a `switch` but exhaustive |
| `&mut dyn Trait` | function params | Mutable reference to "some type implementing this trait" |
| `.expect(msg)` | `ReqwestHttpClient::new()` | Panics with `msg` if the `Result` is `Err` |
| `.parse()` | (called from cli.rs) | Parses a string into another type |
| `enum` | `CommandKind` | A sum type with a fixed set of variants |
| `String::clone()` | `args.action.clone()` | Allocates a new `String` with the same contents |
| `std::process::exit()` | end of `main()` | Terminates the program with an exit code |
