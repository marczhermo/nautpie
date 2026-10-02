# `src/options.rs` — The Options Bag

This file defines a single type, `Options`, that acts as a **key-value store** for parsed CLI options. It's a tiny abstraction, but it's central to how the action functions receive their inputs.

---

## The Big Picture

Here's the chain of how data flows from the command line into an action function:

1. User types `nautpie deploy:naut createDeployment --stack=foo --environment=uat`.
2. `clap` parses that into a `DeployNautArgs` struct (in `cli.rs`).
3. `main.rs` calls `options_from_deploy_args(args)` (in `deploy_naut.rs`), which produces an `Options`.
4. `main.rs` stuffs the `Options` into the `Io` impl via `io.set_options(...)`.
5. The action function reads it back via `io.options()`.

The reason for step 4 — stuffing options through the `Io` trait — is so the `Command::run` trait method can take a uniform `&mut dyn Io` parameter without needing to thread the original CLI args through every layer.

This file defines the bag itself.

---

## The Whole File

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Options {
    values: HashMap<String, String>,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    pub fn option(&self, name: &str) -> Option<String> {
        self.get(name)
    }
}
```

That's the whole file. Let's break down every line.

---

## Walkthrough

### The struct

```rust
#[derive(Debug, Clone, Default)]
pub struct Options {
    values: HashMap<String, String>,
}
```

- **`HashMap<String, String>`** — Rust's standard hash map, imported from `std::collections`. It maps string keys to string values. This is the underlying storage for all parsed CLI options.
- **`#[derive(Debug)]`** — so you can print it with `{:?}` for debugging.
- **`#[derive(Clone)]`** — so you can `.clone()` an `Options` value. The `Io` trait's `options()` method returns `Option<Options>`, which means cloning is necessary when handing it back to callers.
- **`#[derive(Default)]`** — generates a `Default::default()` that produces an empty `Options`. This is what `Options::new()` calls.

Note the field is **not** `pub` — it's private. Callers must use the `set`, `get`, and `option` methods. This is good encapsulation: the internal representation can change (e.g., to a `BTreeMap` for deterministic ordering) without breaking callers.

### `new()`

```rust
pub fn new() -> Self {
    Self::default()
}
```

A trivial constructor. Returns an empty `Options`. The `Self::default()` call uses the `Default` trait derived above. This pattern (`new()` calls `default()`) is idiomatic in Rust.

### `set()`

```rust
pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
    self.values.insert(key.into(), value.into());
}
```

This is the most interesting method because of the parameter types.

- **`&mut self`** — mutable borrow. We're modifying the `Options`, so the caller must also have a mutable reference. (In practice, the only place that calls `set` is `options_from_deploy_args`, which builds a fresh `Options` and so has full ownership.)
- **`key: impl Into<String>`** — `impl Trait` syntax. This says: "I don't care what concrete type you pass, as long as it can be converted into a `String`." So `set("name", "value")` works because `&str` implements `Into<String>` (it allocates and copies). `set(String::from("name"), String::from("value"))` also works. `set(format!("k-{}", 1), "v")` works too. This is **maximum flexibility** for callers.
- **`.insert(key.into(), value.into())`** — convert both arguments into `String` and put them in the hash map. `HashMap::insert` will overwrite any existing value for the same key.

**Why `impl Into<String>` and not just `String`?** Because most callers in `deploy_naut.rs` are passing `Option<String>` results. They have to do `if let Some(v) = &args.url { opts.set("url", v.clone()); }`. Without the `Into` bound, they'd have to write `v.clone()` explicitly; with it, they could pass `v` directly (since `&String` implements `Into<String>`). It saves a small amount of boilerplate at every call site.

### `get()`

```rust
pub fn get(&self, key: &str) -> Option<String> {
    self.values.get(key).cloned()
}
```

- **`&self`** — immutable borrow. We're reading, not modifying.
- **`key: &str`** — borrowed string slice. Callers don't need to pass owned `String`s.
- **`self.values.get(key)`** — returns `Option<&String>` (a reference to the value, or `None`).
- **`.cloned()`** — converts `Option<&String>` to `Option<String>` by cloning the inner value. This is necessary because returning a reference would tie the returned `Option<String>` to the lifetime of `self`, which is a hassle. Cloning is cheap for short strings.

### `option()`

```rust
pub fn option(&self, name: &str) -> Option<String> {
    self.get(name)
}
```

A thin alias for `get()`. Why have both?

- `get` is the **generic, hash-map-flavoured** name.
- `option` is the **domain-flavoured** name — it's how the action functions in `deploy_naut.rs` read options.

It exists because the PHP original had a method called `option()` on its helper class. The Rust port keeps the same call style for parity: `opts.option("stack")` rather than `opts.get("stack")`. Functionally they're identical; the rename is purely cosmetic.

---

## How It's Used

In `deploy_naut.rs`:

```rust
pub fn options_from_deploy_args(args: &DeployNautArgs) -> Options {
    let mut opts = Options::new();
    if let Some(v) = &args.url {
        opts.set("url", v.clone());
    }
    // ... many more like this ...
    opts
}
```

And in an action function:

```rust
let stack = opts
    .option("stack")
    .ok_or_else(|| Error::MissingOption("stack".into()))?;
```

The pattern is: `opts.option("...")` returns `Option<String>`, then `.ok_or_else(|| Error::MissingOption("..."))` converts `None` to an error, and `?` propagates that error up to the caller.

---

## Why Is It Written This Way?

- **Uniform access pattern.** Every action function reads its inputs the same way: `opts.option("...")`. They don't need to know which subcommand they're part of.
- **Late binding.** Options are stuffed into the `Io` impl in `main.rs`, not constructed inside the action. This decouples the action from the CLI parser — in principle, the same action could be triggered by a different input source (env vars, a config file) without changing its code.
- **Encapsulation.** The internal `HashMap` is private. Tomorrow's maintainer could swap it for a `BTreeMap` (sorted) or a `Vec<(String, String)>` (preserving insertion order) without touching any caller.
- **PHP parity.** The original `getOptions()` accessor on the PHP helper class behaved the same way: string in, string out, no validation. The Rust port mirrors that minimalism.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `HashMap<K, V>` | `values` field | Standard library hash map |
| `#[derive(Default)]` | struct | Generates `Default::default()` |
| `#[derive(Clone)]` | struct | Generates `Clone::clone()` |
| `impl Into<T>` | `set` params | "Accepts anything convertible into `T`" |
| `.into()` | inside `set` | Performs the `Into` conversion |
| `Option<T>` | return type | "Some value, or no value" |
| `.cloned()` | `get` | Convert `Option<&T>` to `Option<T>` |
| `Self::default()` | `new` | Calls the derived `Default` impl |
| `&mut self` | `set` | Mutable borrow — the method modifies state |
| `&self` | `get`, `option` | Immutable borrow — the method only reads |
| `impl Trait` in arg position | `set` | Generic over a concrete unknown type |
| Field privacy | `values` | No `pub` means the field is private |
