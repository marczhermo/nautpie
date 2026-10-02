# `src/env.rs` — Working with Environment Variables

This file is a thin wrapper around three small but important operations:

1. **`load_dotenv()`** — read a `.env` file from the current directory.
2. **`check_envs()`** — verify that a set of required environment variables are set, and return their values.
3. **`is_production()`** — check whether a given environment name refers to production.

It's deliberately tiny. Each function is a few lines, and there are no fancy abstractions.

---

## Walkthrough

### `load_dotenv()`

```rust
/// Load `.env` from the current directory. Missing files are ignored, matching
/// PHP `safeLoad`. Returns `true` if a `.env` was loaded, `false` otherwise.
pub fn load_dotenv() -> bool {
    dotenvy::dotenv().is_ok()
}
```

The whole function is one line. Let's unpack it:

- **`dotenvy::dotenv()`** — searches the current directory (and walks up the directory tree) for a file named `.env`. If found, it reads each line of the form `KEY=value` and sets the corresponding environment variable. It returns `Result<(), Error>`.
- **`.is_ok()`** — a method on `Result` that returns `true` if the `Result` is `Ok(_)`, `false` if it's `Err(_)`. We don't care about the specifics of success or failure — only whether anything happened.
- **`-> bool`** — the return type. The function tells its caller "did a `.env` file get loaded?" but the caller (in `main.rs`) **deliberately ignores even this**: `let _ = load_dotenv();`. Why? Because the PHP original's `safeLoad` was also fire-and-forget — it never threw on a missing `.env`.

**Why is the missing-file case ignored?** Because `.env` files are a **developer convenience**, not a deployment requirement. In CI or production, you'd set real environment variables on the machine. If `.env` is missing, the program should still work using real env vars.

### `check_envs()`

```rust
pub fn check_envs(names: &[&str]) -> Result<Vec<String>, Error> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let value = std::env::var(name).map_err(|_| Error::MissingEnv((*name).into()))?;
        if value.is_empty() {
            return Err(Error::MissingEnv((*name).into()));
        }
        out.push(value);
    }
    Ok(out)
}
```

This function takes a **slice of string references** (`&[&str]`) and returns either a `Vec<String>` (one value per requested variable) or an `Error`.

**Teaching points:**

- **`&[&str]`** — this is a slice. The outer `&` means "borrowed reference." The inner `&str` means "borrowed string." So we're borrowing a list of borrowed strings. The caller (in `deploy_naut.rs`) writes `check_envs(&["DASH_USER", "DASH_TOKEN"])`.
- **`Vec::with_capacity(names.len())`** — creates an empty vector that has reserved space for `names.len()` elements. This avoids the vector needing to grow (and reallocate) as we push to it.
- **`std::env::var(name)`** — looks up an environment variable by name. Returns `Result<String, VarError>`. The `?` operator on the next line says: "if it's an `Err`, return that error early from this function."
- **`.map_err(|_| Error::MissingEnv((*name).into()))?`** — this is a **closure** (a tiny inline function). It converts the standard library's error type into our own `Error::MissingEnv`. The `|_|` means "I take one argument but I don't care to name it." The `(*name).into()` converts `&String` (since `name` is `&&str`, dereferencing once gives `&str`) to `String`.
- **`if value.is_empty()`** — PHP's `$_SERVER[$name]` returns `""` for set-but-empty values, while `getenv()` returns `false`. The Rust port treats both cases as "missing" because neither is useful.
- **`out.push(value)`** — appends to the vector. `value` is moved, not cloned.
- **`Ok(out)`** — successful return.

**Why return all values in a `Vec`?** Because the caller usually wants them as a tuple: `let creds = check_envs(&["DASH_USER", "DASH_TOKEN"])?; let user = &creds[0]; let token = &creds[1];`. Returning a `Vec` keeps the function generic over how many env vars you ask for.

### `is_production()`

```rust
pub fn is_production(env: &str) -> bool {
    matches!(env.to_ascii_lowercase().as_str(), "prod" | "production")
}
```

This is a one-liner but it teaches two nice patterns:

- **`matches!(expr, pattern)`** — a macro that returns `true` if the expression matches the pattern, `false` otherwise. Cleaner than writing `match env { ... }` for a single boolean check.
- **`.to_ascii_lowercase().as_str()`** — converts `&str` to a `String` (owned, lowercase), then `.as_str()` borrows it back as `&str`. We need this because we can't pattern-match against the result of a method call directly.
- **`"prod" | "production"`** — a **pattern OR**. Either match works.

The function is used in `deploy_naut.rs` to decide whether to apply the `bypass_and_start` and `redeploy` flags. The reasoning: **you should never bypass or redeploy production deployments**. Those flags are silently ignored if `environment == "prod"` or `"production"`.

---

## Why Is It Written This Way?

- **PHP parity.** The PHP original used `vlucas/phpdotenv` with `safeLoad`, which silently ignores missing files. The Rust port preserves that exact behavior so the migration is invisible.
- **Fail fast.** `check_envs` returns `Error::MissingEnv` at the **first** unset variable, not after collecting all of them. This produces clearer error messages: "X is missing" rather than "X, Y, and Z are missing."
- **Treat empty as missing.** This matches PHP's loose semantics where `""` and `null` are both "no value."
- **`is_production` is case-insensitive.** Users (and CI scripts) sometimes write `PROD`, sometimes `prod`, sometimes `Production`. All should be recognised.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `-> bool` | `load_dotenv` | Function return type |
| `.is_ok()` | `load_dotenv` | "Is this `Result` an `Ok`?" |
| `&[&str]` | `check_envs` param | A borrowed slice of borrowed strings |
| `Vec::with_capacity(n)` | `check_envs` | Pre-allocates space for `n` elements |
| `Result<T, E>` | `check_envs` return | The standard success-or-failure type |
| `?` operator | `check_envs` | "If `Err`, return early with that error" |
| `.map_err(closure)` | `check_envs` | Convert one error type to another |
| Closure `\|_|` | `check_envs` | An inline function with a single unnamed argument |
| `.into()` | `check_envs` | Type conversion via `From`/`Into` |
| `matches!(expr, pattern)` | `is_production` | Pattern-match an expression in one line |
| `"a" \| "b"` | `is_production` | Pattern OR — match either |
| `.to_ascii_lowercase()` | `is_production` | Returns a new `String` in lowercase |
| `.as_str()` | `is_production` | Borrow a `String` back as `&str` |
| `#[cfg(test)]` | tests | Only compile in test mode |
