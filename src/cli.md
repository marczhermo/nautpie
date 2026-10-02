# `src/cli.rs` — Defining the Command-Line Interface

This file uses the `clap` crate to define the CLI surface. When you type `nautpie --help`, the help text you see comes from the structures defined here. When you type `nautpie deploy:naut createDeployment --stack=foo`, the parsing of that string is done by the code here.

The single most important idea in this file is that **`clap` generates the parser at compile time** from the structs and attributes you write. There's no separate "schema" file and no runtime parsing — you write Rust structs and `clap` figures out the rest.

---

## The Big Picture

This file defines:

1. **`normalise_action()`** — a helper that converts any case style (`sampleSuccess`, `Sample-Success`, `SAMPLE_SUCCESS`) into the canonical `lowerCamelCase`.
2. **`Cli`** — the top-level struct that says "this program takes a subcommand."
3. **`CommandKind`** — an enum with two variants: `DeployNaut` and `Bitbucket`. Each variant holds its own argument struct.
4. **`DeployNautArgs`** — every flag the `deploy:naut` subcommand accepts.
5. **`BitbucketArgs`** — every flag the `ci:bitbucket` subcommand accepts.

When `Cli::parse()` runs (in `main.rs`), `clap` walks the arguments and produces a fully-populated `Cli` value.

---

## Walkthrough

### `normalise_action()`

```rust
pub fn normalise_action(raw: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut prev_is_lower = false;
    for c in raw.chars() {
        if c == '_' || c == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            prev_is_lower = false;
            continue;
        }
        let upper = c.is_ascii_uppercase();
        if upper && prev_is_lower {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
        prev_is_lower = c.is_ascii_lowercase();
    }
    if !current.is_empty() {
        words.push(current);
    }
    // ... join logic ...
}
```

This is a small string-processing function, but it teaches several Rust concepts:

- **`&str`** — a borrowed string slice. We take a reference (`&`) so we don't have to take ownership of the caller's string.
- **`String::new()`** — creates an empty, growable string on the heap. The opposite would be `""` (a string literal, which is immutable and stored in the program's read-only data segment).
- **`Vec<String>`** — a growable list of owned strings. The `Vec!` macro (with the `!`) creates an empty vec.
- **`std::mem::take(&mut current)`** — a handy trick. It replaces `current` with the **default value** (here, an empty `String`) and returns the old value. So we can move the accumulated characters out of `current` into the `words` vector without cloning them. This is more efficient than `current.clone()`.
- **`c.is_ascii_uppercase()`** — a method on `char` (Rust's 32-bit Unicode scalar type). It returns `true` if the character is in the ASCII range A–Z.
- **`continue`** — skips the rest of the loop body and goes to the next iteration.

The algorithm walks the input character by character. When it sees an underscore or hyphen, that's a word boundary. When it sees an uppercase letter preceded by a lowercase letter (`aB` → split between `a` and `B`), that's another word boundary. Everything else accumulates into `current`.

After collecting the words, it joins them: the first word is lowercased (`sample`), and each subsequent word is capitalised (`Success`), giving `sampleSuccess`.

### `Cli` and the `Parser` derive

```rust
#[derive(Debug, Parser)]
#[command(
    name = "nautpie",
    version,
    about = "DeployNaut / Bitbucket Pipelines deployment client"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CommandKind,
}
```

- **`#[derive(Debug, Parser)]`** — These are **derive macros**. `Debug` makes the struct printable for debugging. `Parser` is `clap`'s magic — it generates a `Cli::parse()` function and a `Cli::try_parse_from(...)` function for you.
- **`#[command(...)]`** — attributes that configure `clap`. `name`, `version`, and `about` populate the `--help` text.
- **`#[command(subcommand)]`** — tells `clap` that this field is a subcommand, so the user must type one of the variants of `CommandKind`.

### `CommandKind` enum

```rust
#[derive(Debug, Subcommand)]
pub enum CommandKind {
    /// DeployNaut API actions.
    #[command(name = "deploy:naut")]
    DeployNaut(DeployNautArgs),

    /// Bitbucket Pipelines actions.
    #[command(name = "ci:bitbucket")]
    Bitbucket(BitbucketArgs),
}
```

This is one of Rust's most powerful features: an **enum with data attached**. Each variant carries a different type:

- `DeployNaut(DeployNautArgs)` — when the user types `deploy:naut`, the parsed arguments go here.
- `Bitbucket(BitbucketArgs)` — when the user types `ci:bitbucket`, the parsed arguments go here.

`#[command(name = "deploy:naut")]` is the literal string the user types. Without it, `clap` would default to the variant name (`deploy-naut` with hyphens), but we want to match the PHP original's colon syntax.

### `SharedArgs` trait

```rust
#[allow(dead_code)]
pub trait SharedArgs {
    fn action(&self) -> &str;
}
```

This is a tiny trait with one method. Both `DeployNautArgs` and `BitbucketArgs` implement it (see below). It's a way of saying: "any struct that represents a subcommand's arguments has an `action` method."

```rust
impl SharedArgs for DeployNautArgs {
    fn action(&self) -> &str {
        &self.action
    }
}
```

This says: "the `DeployNautArgs` struct implements `SharedArgs`, and the `action` method returns a reference to its `action` field."

**Why bother?** It lets generic code write `args.action()` without caring whether `args` is `DeployNautArgs` or `BitbucketArgs`. Today the code uses each type directly, so `SharedArgs` is currently more aspirational than load-bearing — but it's there for future flexibility.

The `#[allow(dead_code)]` attribute suppresses the warning that the trait is currently unused (only its methods are called through concrete types).

### The argument structs

```rust
#[derive(Debug, clap::Args)]
pub struct DeployNautArgs {
    /// Command action (e.g. `sampleSuccess`, `createDeployment`).
    #[arg(value_name = "ACTION")]
    pub action: String,

    /// [Optional] URL
    #[arg(long)]
    pub url: Option<String>,

    /// [Optional] Git commit SHA
    #[arg(long)]
    pub commit: Option<String>,

    // ... many more ...
}
```

**Teaching points:**

- **`#[derive(clap::Args)]`** — marks this as a "leaf" group of arguments that doesn't have its own subcommands.
- **`pub action: String`** — the positional argument (no `--` prefix). `value_name = "ACTION"` is just the placeholder name shown in `--help`.
- **`pub url: Option<String>`** — `Option<String>` means "either `Some(value)` or `None`." This is how `clap` represents optional arguments. The user might not have supplied `--url`, in which case the field is `None`.
- **`#[arg(long)]`** — the long flag form: `--url`, `--commit`, etc. There are also `#[arg(short)]` for `-u` and `#[arg(long, short)]` for both.
- **The `///` comments** are **doc comments**. `clap` picks them up and shows them in `--help`. The first line becomes the description.

### One subtle thing: `r#ref`

```rust
/// [Optional] Deployment Reference
#[arg(long)]
pub r#ref: Option<String>,
```

Notice the strange `r#` prefix. In Rust, `ref` is a **keyword** (used for taking references in patterns). You can't use a reserved word as a field name directly. The `r#` prefix is called a **raw identifier** — it tells the compiler "I know this looks like a keyword, but treat it as a plain identifier."

Why did the original authors need `ref` as a field name? Because that's exactly the option name in DeployNaut's API: `ref` is the Git reference (branch, tag, or commit SHA) being deployed.

### Tests

The `#[cfg(test)]` block at the bottom only gets compiled when you run `cargo test`. It's removed from release builds entirely (`cfg(test)` is `false` in production).

The tests use `Cli::try_parse_from(["nautpie", "deploy:naut", "sampleSuccess"])` — passing the arguments as a slice of strings instead of reading them from `std::env::args()`. This is how you test a `clap` parser without actually invoking the binary.

---

## Why Is It Written This Way?

- **Derive macros over manual parsing.** `clap` generates a robust, well-tested parser at compile time. Writing it by hand would be hundreds of lines and full of edge cases.
- **Case-insensitive action names.** The PHP version accepted `sampleSuccess`, `SampleSuccess`, or `SAMPLESUCCESS` interchangeably (because PHP method calls are case-insensitive). Rust method calls are case-sensitive, so we normalise the input to one canonical form before dispatch. The `normalise_action()` function exists entirely to preserve that user-facing behavior.
- **Option<String> everywhere.** PHP allowed arguments to be missing without complaint — they'd just be `null`. In Rust, "missing" is best modelled as `Option<T>::None`. Every optional flag uses this type so the action functions can pattern-match cleanly.
- **Two argument structs sharing a trait.** Most flags differ between subcommands (`deploy:naut` has `--ref_type`, `ci:bitbucket` doesn't). Rather than one giant struct with all flags, we have two focused ones. The `SharedArgs` trait is the thin glue between them.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `derive(Parser)` | `Cli` | Auto-generates argument parsing |
| `derive(Subcommand)` | `CommandKind` | Auto-generates subcommand dispatch |
| `derive(clap::Args)` | `DeployNautArgs` | Marks a leaf argument group |
| `derive(Debug)` | every struct/enum | Makes the type printable with `{:?}` |
| `enum` with data | `CommandKind` | A type that can be one of several variants, each holding its own data |
| `Option<T>` | optional flags | "Some value, or no value at all" |
| `&str` | function params | A borrowed view into a string |
| `String` | fields | An owned, growable string |
| `Vec<T>` | inside `normalise_action` | A growable list |
| `r#ref` | field name | Raw identifier — escape a reserved word |
| `#[arg(...)]` | field attributes | Configures how `clap` parses that field |
| `#[command(...)]` | struct/enum attributes | Configures how `clap` treats the whole group |
| `#[cfg(test)]` | tests module | "Only compile this in test mode" |
| `impl Trait for Type` | `SharedArgs` impls | Implementing a trait for a specific type |
| `trait` | `SharedArgs` | A set of methods a type must implement |
| Doc comments `///` | field comments | Documentation that `clap` and `rustdoc` both read |
