# `src/tui/spinner.rs` — The Progress Polling Loop

This is the **most algorithmically complex** file in the codebase. It implements a polling loop that:

1. Repeatedly calls a callback (which usually hits an HTTP endpoint).
2. Renders the current state to the terminal.
3. Stops when the callback signals completion, the callback returns an error, or a deadline expires.

It has two implementations: one for **TTY** environments (interactive terminals) using `ratatui`, and one for **non-TTY** (CI logs) using plain stderr lines.

---

## The Big Picture

Two actions in `deploy_naut.rs` use this:

- `gitFetch` — polls `/project/{stack}/git/fetches/{id}` every 5 seconds until the git fetch completes.
- `checkDeploymentProgress` — polls `/project/{stack}/environment/{env}/deploys/{id}` until the deployment reaches the `Completed` state.

Both pass a deadline (2 minutes for git, 30 minutes for deploy), a label, and a closure that performs the polling HTTP request.

The `ProgressRunner` is the workhorse. It:

1. Checks if stderr is a TTY.
2. If yes: enters `run_tui` (ratatui-based).
3. If no: enters `run_fallback` (stderr-line-based).
4. Either way, polls until done / errored / timed out.

---

## Walkthrough

### Imports

```rust
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::error::Error;
use crate::http::HttpClient;
```

- `std::io::{self, IsTerminal, Write}` — `io` (for `stderr()`), `IsTerminal` (a trait with `.is_terminal()`), `Write` (for `.writeln!`).
- `std::time::{Duration, Instant}` — `Duration` for time spans, `Instant` for measuring elapsed time.
- `ratatui::*` — the various pieces needed for TUI rendering.
- `crate::error::Error` and `crate::http::HttpClient` — our own types.

### The `PollOutcome` enum

```rust
pub enum PollOutcome {
    Pending(String),
    Done,
}
```

The result of one polling iteration:

- **`Pending(String)`** — not done yet. The `String` is the current status (e.g., `"Queued"`, `"Deploying"`) for display.
- **`Done`** — operation completed.

This is a **simple enum** (no fields on `Done`, just a tuple variant on `Pending`).

### The `PollFn` type alias

```rust
pub type PollFn = Box<dyn FnMut(&mut dyn HttpClient) -> Result<PollOutcome, Error>>;
```

A type alias for "a boxed, mutable closure that takes an HTTP client and returns a `PollOutcome` or error."

**Teaching points:**

- **`type Alias = ...;`** — defines a new name for an existing type. `PollFn` and `Box<dyn FnMut(...)>` are interchangeable.
- **`Box<dyn Trait>`** — a heap-allocated trait object. `dyn FnMut(...)` is a closure that mutates captured state.
- **`FnMut`** — closures come in three flavors:
  - `Fn` — borrows captured state immutably.
  - `FnMut` — borrows captured state mutably.
  - `FnOnce` — consumes captured state.

  Polling closures need to be `FnMut` because they're called multiple times. The HTTP client is mutated on each call.

### The `ProgressRunner` struct

```rust
pub struct ProgressRunner {
    pub title: &'static str,
    pub label: String,
    pub poll_interval: Duration,
    pub tick_rate: Duration,
}
```

Configuration for one polling run.

**Teaching points:**

- **`pub title: &'static str`** — the title shown in the TUI box (e.g., `"gitFetch"`). `&'static str` is required because `ProgressRunner::new` stores string literals.
- **`pub label: String`** — a longer description, e.g., `"Git fetch for example-stack (#42)"`. Owned because it's computed dynamically.
- **`pub poll_interval: Duration`** — how often to call the poll function. Defaults to 5 seconds (PHP parity).
- **`pub tick_rate: Duration`** — how often to redraw the TUI. Defaults to 250 milliseconds (4 frames per second).
- All fields are `pub`. This is unusual — normally Rust prefers private fields with accessor methods. But here, the fields are intended to be tweaked after construction. For example, the tests access `runner.title` directly.

### `new()`

```rust
pub fn new(title: &'static str, label: impl Into<String>) -> Self {
    Self {
        title,
        label: label.into(),
        poll_interval: Duration::from_secs(5),
        tick_rate: Duration::from_millis(250),
    }
}
```

**Teaching points:**

- **`label: impl Into<String>`** — accepts anything convertible to `String` (e.g., `&str`, `String`, `format!(...)`).
- **`.into()`** — performs the conversion.
- **Defaults** match PHP behavior (5-second polling) and a smooth UI (4 frames per second).

### `run()` — the dispatcher

```rust
pub fn run(self, deadline: Duration, poll_fn: PollFn, http: &mut dyn HttpClient) -> Result<(), Error> {
    if io::stderr().is_terminal() {
        self.run_tui(deadline, poll_fn, http)
    } else {
        self.run_fallback(deadline, poll_fn, http)
    }
}
```

Picks the implementation based on whether stderr is a TTY.

**Teaching points:**

- **`self`** (consuming) — by taking `self` by value (not `&self`), the method **consumes** the runner. After `run()` is called, the runner is gone. This prevents reusing the same runner with different configs (which would be a likely source of bugs).
- **`io::stderr().is_terminal()`** — the `IsTerminal` trait, stable since Rust 1.70. Returns `true` if stderr is connected to a terminal (not piped to a file or another program).
- **Branch to either `run_tui` or `run_fallback`.** The two functions handle the same polling logic but render output differently.

### `run_tui()` — the ratatui-based loop

```rust
fn run_tui(self, deadline: Duration, mut poll_fn: PollFn, http: &mut dyn HttpClient) -> Result<(), Error> {
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => return self.run_fallback(deadline, poll_fn, http),
    };
    let _ = ratatui::crossterm::terminal::enable_raw_mode();
    let _ = terminal.clear();

    let start = Instant::now();
    let mut last_status = String::new();
    let mut next_poll = Instant::now();
    let mut next_tick = Instant::now();

    loop {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            return Err(Error::Timeout(format!(
                "{} timed out after {}s",
                self.label,
                elapsed.as_secs()
            )));
        }

        if Instant::now() >= next_poll {
            match poll_fn(http) {
                Ok(PollOutcome::Pending(status)) => { last_status = status; }
                Ok(PollOutcome::Done) => {
                    let _ = ratatui::crossterm::terminal::disable_raw_mode();
                    return Ok(());
                }
                Err(e) => {
                    let _ = ratatui::crossterm::terminal::disable_raw_mode();
                    return Err(e);
                }
            }
            next_poll = Instant::now() + self.poll_interval;
        }

        if Instant::now() >= next_tick {
            let _ = terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
                    .split(f.area());

                let title = Paragraph::new(Line::from(vec![Span::styled(
                    self.title,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]))
                .block(Block::default().borders(Borders::ALL).title(" nautpie "));
                f.render_widget(title, chunks[0]);

                let body = Paragraph::new(vec![
                    Line::from(format!("Label : {}", self.label)),
                    Line::from(format!("Elapsed : {}s", elapsed.as_secs())),
                    Line::from(format!("Status : {}", last_status)),
                ])
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(body, chunks[1]);
            });
            next_tick = Instant::now() + self.tick_rate;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
```

This is the meatiest function. Let's break it down.

#### Terminal bring-up

```rust
let backend = CrosstermBackend::new(io::stderr());
let mut terminal = match Terminal::new(backend) {
    Ok(t) => t,
    Err(_) => return self.run_fallback(deadline, poll_fn, http),
};
```

**Teaching points:**

- **`CrosstermBackend::new(io::stderr())`** — ratatui uses a "backend" abstraction for terminal I/O. `CrosstermBackend` wraps a `Write` implementor (here, stderr) and turns it into a TTY-renderable surface.
- **`Terminal::new(backend)`** — wrap the backend in a `Terminal` that can be drawn to. Returns `Result`.
- **The fallback `Err(_) => return self.run_fallback(...)`** — if terminal initialization fails (e.g., TERM env var missing on CI), gracefully fall back to the non-TUI mode.

#### Raw mode

```rust
let _ = ratatui::crossterm::terminal::enable_raw_mode();
```

**Teaching points:**

- **Raw mode** disables line buffering and echo so the TUI can take over the entire screen. It must be **disabled** before returning, otherwise the terminal stays in a broken state.
- **`let _ = ...`** — we ignore the result. Raw mode might fail in some environments; if it does, we just continue without it.

#### Time state

```rust
let start = Instant::now();
let mut last_status = String::new();
let mut next_poll = Instant::now();
let mut next_tick = Instant::now();
```

Four pieces of mutable state:

- **`start`** — when the loop began. Used to compute elapsed time.
- **`last_status`** — the most recent status string from the polling callback. Displayed in the TUI.
- **`next_poll`** — when the next poll should run. We check `Instant::now() >= next_poll` to decide.
- **`next_tick`** — when the next TUI redraw should happen. Same pattern.

This is a classic **fixed-timestep loop** pattern from game development: separate the "polling rate" (5s) from the "render rate" (250ms).

#### The main loop

```rust
loop {
    let elapsed = start.elapsed();
    if elapsed >= deadline {
        // timeout handling
    }

    if Instant::now() >= next_poll {
        // poll
    }

    if Instant::now() >= next_tick {
        // draw
    }

    std::thread::sleep(Duration::from_millis(50));
}
```

Three phases per iteration:

1. **Check deadline.** If we've exceeded it, disable raw mode and return `Timeout`.
2. **Poll if it's time.** Run the callback; update `last_status` or return early on `Done`/`Err`.
3. **Draw if it's time.** Re-render the TUI frame.
4. **Sleep 50ms.** Prevents the loop from spinning at 100% CPU.

#### Polling

```rust
if Instant::now() >= next_poll {
    match poll_fn(http) {
        Ok(PollOutcome::Pending(status)) => { last_status = status; }
        Ok(PollOutcome::Done) => {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            return Ok(());
        }
        Err(e) => {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            return Err(e);
        }
    }
    next_poll = Instant::now() + self.poll_interval;
}
```

**Teaching points:**

- **`poll_fn(http)`** — invokes the closure passed in by the caller. It mutably borrows `http`, makes a request, and returns a `Result<PollOutcome, Error>`.
- **The three-arm match** handles all three possible outcomes.
- **On every exit, we disable raw mode.** This is critical — leaving the terminal in raw mode would corrupt subsequent output.
- **`next_poll = Instant::now() + self.poll_interval`** — schedule the next poll 5 seconds from now.

#### Drawing

```rust
if Instant::now() >= next_tick {
    let _ = terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
            .split(f.area());

        let title = Paragraph::new(Line::from(vec![Span::styled(
            self.title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]))
        .block(Block::default().borders(Borders::ALL).title(" nautpie "));
        f.render_widget(title, chunks[0]);

        let body = Paragraph::new(vec![
            Line::from(format!("Label : {}", self.label)),
            Line::from(format!("Elapsed : {}s", elapsed.as_secs())),
            Line::from(format!("Status : {}", last_status)),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(body, chunks[1]);
    });
    next_tick = Instant::now() + self.tick_rate;
}
```

This is **ratatui boilerplate**. The pattern is:

1. Split the screen into chunks (here: two vertical sections of 3 lines each).
2. Create a `Paragraph` widget with the text to display.
3. Wrap it in a `Block` with a border.
4. Render the widget into the chunk.

**Teaching points:**

- **`terminal.draw(|f| { ... })`** — the closure receives a `Frame` (`f`), which represents the current screen. We draw onto it.
- **`f.area()`** — the full area of the frame. We split it.
- **`Constraint::Length(3)`** — each chunk is 3 lines tall.
- **`[Constraint::Length(3), Constraint::Length(3)].as_ref()`** — the constraints array. `.as_ref()` converts `&[Constraint; 2]` to `&[Constraint]`.
- **`Span::styled(text, style)`** — a styled piece of text. `Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)` produces a bold cyan text style.
- **`Paragraph::new(line).block(border).render(area)`** — the standard pattern for a framed text widget.

#### The sleep

```rust
std::thread::sleep(Duration::from_millis(50));
```

A 50ms sleep throttles the loop. Without it, the loop would spin as fast as the CPU allows, wasting energy and heat.

### `run_fallback()` — the non-TUI loop

```rust
fn run_fallback(self, deadline: Duration, mut poll_fn: PollFn, http: &mut dyn HttpClient) -> Result<(), Error> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "[nautpie] {} …", self.label);

    let start = Instant::now();
    let mut next_poll = Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            // timeout
        }

        if Instant::now() >= next_poll {
            let _ = writeln!(handle, "[nautpie] Waiting… ({}s)", elapsed.as_secs());
            match poll_fn(http) {
                Ok(PollOutcome::Pending(status)) => { let _ = writeln!(handle, "[nautpie] Status: {status}"); }
                Ok(PollOutcome::Done) => return Ok(()),
                Err(e) => return Err(e),
            }
            next_poll = Instant::now() + self.poll_interval;
        }

        std::thread::sleep(Duration::from_millis(200));
    }
}
```

Same logic as `run_tui`, but:

- **No terminal setup or raw mode** — just lock stderr once and use it.
- **No TUI rendering** — just `writeln!` to print lines like `[nautpie] Waiting… (30s)` and `[nautpie] Status: Queued`.
- **200ms sleep** instead of 50ms — less responsive but cheaper on CI.

The output looks like:

```
[nautpie] Git fetch for example-stack (#42) …
[nautpie] Waiting… (0s)
[nautpie] Waiting… (5s)
[nautpie] Status: In Progress
[nautpie] Waiting… (10s)
[nautpie] Status: Complete
```

This matches PHP's behavior: a line every 5 seconds with the current state.

### `run_with_progress()` — convenience wrapper

```rust
pub fn run_with_progress<F>(
    title: &'static str,
    label: impl Into<String>,
    deadline: Duration,
    http: &mut dyn HttpClient,
    poll_fn: F,
) -> Result<(), Error>
where
    F: FnMut(&mut dyn HttpClient) -> Result<PollOutcome, Error> + 'static,
{
    let runner = ProgressRunner::new(title, label);
    let boxed: PollFn = Box::new(poll_fn);
    runner.run(deadline, boxed, http)
}
```

A free function that wraps the boilerplate:

1. Build a `ProgressRunner` with the given title and label.
2. Box the closure into a `PollFn`.
3. Run it.

**Teaching points:**

- **The `where F: FnMut(...) + 'static`** — generic over any closure-like type. The `'static` bound means the closure can't borrow non-`'static` data (it must own everything it captures).
- **`Box::new(poll_fn)`** — heap-allocate the closure. The `Box<dyn FnMut(...)>` is what `PollFn` is.

In `deploy_naut.rs`, callers write:

```rust
run_with_progress(
    "gitFetch",
    format!("Git fetch for {stack} (#{fetch_id})"),
    Duration::from_secs(GIT_TIMEOUT_SECS),
    http,
    move |http| poll_status(http, &poll_url_for_closure, "status"),
)?;
```

The `move` keyword is required because the closure outlives the calling function.

### Tests

The tests verify:

1. **Default constructor values** are correct.
2. **Timeout works** — a never-Done poll produces `Error::Timeout`.
3. **Immediate Done works** — a poll that returns `Done` succeeds immediately.

Each test creates a `NoopClient` (a fake `HttpClient` impl) so no real HTTP calls happen.

---

## Why Is It Written This Way?

- **TTY-aware UX.** On a developer's machine, you get a smooth live-updating TUI. On CI, you get simple log lines. The same code path picks the right one based on `is_terminal()`.
- **Fixed-timestep loop.** Decoupling poll rate (5s) from render rate (250ms) means we don't make HTTP calls faster than necessary while still keeping the UI smooth.
- **Graceful degradation.** If `Terminal::new` fails (e.g., on a broken CI runner), we fall back to the stderr-line variant. The user gets progress either way.
- **Two return paths share cleanup.** Both `run_tui` and the early-exit cases in the polling match disable raw mode before returning. This is critical for not corrupting the terminal.
- **PHP parity.** The 5-second polling interval matches PHP's `sleep(5)` loop.

---

## Key Rust Concepts Used Here

| Concept | Where | What it does |
|---|---|---|
| `enum PollOutcome` | type | Two-variant result enum |
| `type Alias = ...;` | `PollFn` | Type alias for readability |
| `Box<dyn FnMut(...)>` | `PollFn` | Heap-allocated mutable closure |
| `FnMut` | `PollFn` | Closure flavor that mutates captured state |
| `+ 'static` | `run_with_progress` bound | Closure can't borrow non-`'static` data |
| `pub struct` | `ProgressRunner` | Data struct with public fields |
| `&'static str` | `title` field | String slice with `'static` lifetime |
| `Duration::from_secs(n)` | defaults | Create a duration from seconds |
| `Duration::from_millis(n)` | sleep | Create a duration from milliseconds |
| `Instant::now()` | timing | Current monotonic time |
| `Instant::now() + Duration` | scheduling | Future instant |
| `start.elapsed()` | timing | Time elapsed since `start` |
| `std::thread::sleep(...)` | loop | Pause for a duration |
| `io::stderr().is_terminal()` | dispatcher | "Is stderr a TTY?" |
| `CrosstermBackend::new(...)` | `run_tui` | ratatui backend over stderr |
| `Terminal::new(backend)` | `run_tui` | ratatui terminal wrapper |
| `terminal.draw(\|f\| ...)` | TUI render | Render a frame |
| `Layout::default().direction(...)` | TUI | Split the screen |
| `Constraint::Length(n)` | TUI | Fixed-size constraint |
| `f.area()` | TUI | The full screen area |
| `Paragraph::new(...)` | TUI | Text widget |
| `Block::default().borders(Borders::ALL)` | TUI | Bordered box |
| `Style::default().fg(Color::Cyan)` | TUI | Text style |
| `Span::styled(...)` | TUI | A styled piece of text |
| `Line::from(...)` | TUI | A line of text |
| `enable_raw_mode()` | `run_tui` | Disable line buffering/echo |
| `disable_raw_mode()` | exits | Restore terminal state |
| `move` closure | callers | Move captured vars into the closure |
| `impl Into<String>` | `label` param | Accept anything convertible to `String` |
| Generic `<F: FnMut(...) + 'static>` | `run_with_progress` | Generic over closure type |
| `Result<(), Error>` | return | Success-or-failure |
| `Error::Timeout(...)` | timeout | Construct the timeout error variant |
| `Box::new(poll_fn)` | `run_with_progress` | Heap-allocate the closure |
| `impl Trait` for params | `run_with_progress` | "Accepts anything implementing this" |
| `where F: Trait` | `run_with_progress` | Trait bound on a generic |
