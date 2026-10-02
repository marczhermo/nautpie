# `src/boolean.rs` — Parsing Booleans from Strings

This is the smallest file in the codebase. It exports **one function** that does **one thing**: convert a string like `"yes"` or `"no"` into a real `bool`. Despite its size, it teaches some genuinely useful Rust patterns.

---

## The Whole File

```rust
pub fn check_boolean(value: Option<&str>) -> Result<bool, Error> {
    match value.map(str::trim).map(str::to_ascii_lowercase) {
        None => Ok(false),
        Some(v) if v.is_empty() => Ok(false),
        Some(v) if matches!(v.as_str(), "false" | "no" | "0") => Ok(false),
        Some(v) if matches!(v.as_str(), "true" | "yes" | "1") => Ok(true),
        Some(v) => Err(Error::Generic(format!(
            "[Boolean] Value not accepted as boolean: {v}"
        ))),
    }
}
```

That's it. One function, one match, five arms.

---

## Walkthrough

### The function signature

```rust
pub fn check_boolean(value: Option<&str>) -> Result<bool, Error>
```

- **`Option<&str>`** — the input is "either a string slice, or nothing." This matches the way `clap` represents optional CLI flags: `Option<String>`. The caller can pass `Some("yes")`, `Some("")`, or `None`.
- **`Result<bool, Error>`** — output is "either a real boolean, or our error type."

### The match

The body is a single `match` expression with five arms. Let's go through it one piece at a time.

### `value.map(str::trim).map(str::to_ascii_lowercase)`

This is the most interesting line in the file, so let's break it down carefully.

- **`Option::map(closure)`** — a method on `Option<T>`. If the `Option` is `Some(x)`, it applies the closure to `x` and returns `Some(result)`. If the `Option` is `None`, it returns `None` unchanged. Think of it as "transform the inside of an `Option`, leaving `None` alone."
- **`str::trim`** — passed as a **function pointer**. The full signature of `str::trim` is `fn trim(&self) -> &str`. When we pass it to `.map()`, Rust uses it like a closure: `|s| s.trim()`. Same effect, shorter syntax.
- **`str::to_ascii_lowercase`** — same trick. Signature: `fn to_ascii_lowercase(&self) -> String`. So `value.map(str::to_ascii_lowercase)` would convert the inner string to lowercase.

The two `.map` calls are **chained**. Each one transforms the value if there is one:

1. First, trim whitespace from the string (e.g. `"  yes  "` → `"yes"`).
2. Then, lowercase the result (e.g. `"YES"` → `"yes"`).

After these two maps, we have:

- `None` — if the input was `None`
- `Some("yes")` — if the input was `Some("yes")`, `Some(" YES ")`, etc.
- `Some("")` — if the input was `Some("")`, `Some("   ")`, etc.

### The match arms

Now we match on the normalised value. Each arm uses a **match guard** — an extra `if` condition after the pattern:

```rust
None => Ok(false),
Some(v) if v.is_empty() => Ok(false),
Some(v) if matches!(v.as_str(), "false" | "no" | "0") => Ok(false),
Some(v) if matches!(v.as_str(), "true" | "yes" | "1") => Ok(true),
Some(v) => Err(Error::Generic(format!(
    "[Boolean] Value not accepted as boolean: {v}"
))),
```

- **`None => Ok(false)`** — no input given? Default to `false`.
- **`Some(v) if v.is_empty() => Ok(false)`** — empty string after trimming? Default to `false`.
- **`Some(v) if matches!(v.as_str(), "false" | "no" | "0") => Ok(false)`** — explicit `false` synonyms. The pattern guard (`if matches!(...)`) refines the `Some(v)` arm so it only matches when the string is one of those three.
- **`Some(v) if matches!(v.as_str(), "true" | "yes" | "1") => Ok(true)`** — explicit `true` synonyms.
- **`Some(v) => Err(...)`** — the **catch-all arm**. Anything that survived the guards but isn't a recognised truthy/falsy string is an error.

**Teaching points:**

- **Pattern guards (`if condition`)** — they let you add extra conditions to a match arm without nesting more `match` or `if` statements. Very useful for matching on a range of values.
- **`matches!(v.as_str(), "true" | "yes" | "1")`** — the same `matches!` macro from `env.rs`. It returns `true` if the expression matches the pattern.
- **`format!("...{v}...")`** — string interpolation. `{v}` is shorthand for `{}` when the variable's name is the same as the placeholder.
- **The order matters.** `Some(v) if v.is_empty()` must come **before** `Some(v) if matches!(v.as_str(), "false" | "no" | "0")` because both have a `Some(v)` pattern — but the empty-string check is more specific (the other patterns wouldn't match an empty string anyway, but ordering it first makes the intent clearer).

### Why `str::trim` and not `value.trim()`?

Both would work. Writing `str::trim` (passing the function reference) is idiomatic when you're chaining transformations through `.map()`. It reads like a pipeline: "take the value, map with trim, map with lowercase."

If you wrote `value.map(|s| s.trim())`, it would be exactly equivalent but longer.

---

## Why Is It Written This Way?

- **PHP `CheckHelper::checkBoolean` parity.** The original PHP function was deliberately lenient: anything missing counts as `false`, empty counts as `false`, a few specific strings count as `true` or `false`, and anything else throws. The Rust port preserves every one of those cases.
- **`Option<&str>` over `&str`.** By taking `Option<&str>`, the caller can pass through `Option::None` directly from `clap` without writing `if let Some(...)` everywhere. The signature matches the shape of the data.
- **Match guards over nested `if`.** Five arms with guards is more readable than one arm with five nested `if-else` blocks. Each arm reads top-to-bottom as a separate case.
- **Custom error message.** `"[Boolean] Value not accepted as boolean: maybe"` is more helpful than a generic error. The `[Boolean]` prefix matches the PHP original's error format.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `Option<&str>` | param | "Either a string slice, or nothing" |
| `Result<bool, Error>` | return | "Either a `bool`, or our error type" |
| `Option::map(closure)` | the chained maps | "Transform the inside of an `Option`" |
| Function pointer `str::trim` | first `.map` | Passing a function as a value |
| Method chaining `.map().map()` | the pipeline | Calling multiple methods in sequence |
| `match` with guards | the body | Pattern matching with extra `if` conditions |
| `matches!(expr, pattern)` | inside guards | "Does this match the pattern?" |
| Pattern OR `\|` | inside `matches!` | "Either this or that" |
| `format!("...{v}...")` | error case | String interpolation |
| Implicit return | every arm | The last expression (no semicolon) is the value |
| `#[cfg(test)]` | tests | Only compile in test mode |
