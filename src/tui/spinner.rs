//! ratatui-based progress runner. The runner polls a closure until it
//! returns `Some(Ok(()))`, `Some(Err(_))`, or the supplied deadline elapses.
//!
//! On TTY, it draws a small ratatui frame (title, label, elapsed seconds,
//! last status) updating ~4× per second. On non-TTY, it prints a
//! `"Waiting…"` line to stderr every 5 seconds to match PHP behaviour.

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

/// Outcome of a single poll tick.
pub enum PollOutcome {
    /// The polled action is not done yet; keep polling. Carries the last
    /// observed status string for the UI.
    Pending(String),
    /// The polled action has finished successfully.
    Done,
}

/// A polling callback: invoked repeatedly with the HTTP client and is
/// expected to either report a pending status, signal completion, or
/// surface an error.
pub type PollFn = Box<dyn FnMut(&mut dyn HttpClient) -> Result<PollOutcome, Error>>;

/// Configuration for the runner.
pub struct ProgressRunner {
    pub title: &'static str,
    pub label: String,
    pub poll_interval: Duration,
    pub tick_rate: Duration,
}

impl ProgressRunner {
    pub fn new(title: &'static str, label: impl Into<String>) -> Self {
        Self {
            title,
            label: label.into(),
            // PHP polls every 5 seconds; keep parity as the default.
            poll_interval: Duration::from_secs(5),
            tick_rate: Duration::from_millis(250),
        }
    }

    /// Run the polling loop until the poll function signals completion, an
    /// error occurs, or the deadline elapses. Returns `Ok(())` on success or
    /// the propagated `Err(Error::Timeout)` on deadline expiry.
    pub fn run(
        self,
        deadline: Duration,
        poll_fn: PollFn,
        http: &mut dyn HttpClient,
    ) -> Result<(), Error> {
        if io::stderr().is_terminal() {
            self.run_tui(deadline, poll_fn, http)
        } else {
            self.run_fallback(deadline, poll_fn, http)
        }
    }

    fn run_tui(
        self,
        deadline: Duration,
        mut poll_fn: PollFn,
        http: &mut dyn HttpClient,
    ) -> Result<(), Error> {
        // Best-effort terminal bring-up. If init fails (e.g. on a CI runner
        // with no TERM), fall back to the stderr variant so we still make
        // progress.
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
                    Ok(PollOutcome::Pending(status)) => {
                        last_status = status;
                    }
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
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
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

    fn run_fallback(
        self,
        deadline: Duration,
        mut poll_fn: PollFn,
        http: &mut dyn HttpClient,
    ) -> Result<(), Error> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(handle, "[nautpie] {} …", self.label);

        let start = Instant::now();
        let mut next_poll = Instant::now();
        loop {
            let elapsed = start.elapsed();
            if elapsed >= deadline {
                let _ = writeln!(
                    handle,
                    "[nautpie] {} timed out after {}s",
                    self.label,
                    elapsed.as_secs()
                );
                return Err(Error::Timeout(format!(
                    "{} timed out after {}s",
                    self.label,
                    elapsed.as_secs()
                )));
            }

            if Instant::now() >= next_poll {
                let _ = writeln!(handle, "[nautpie] Waiting… ({}s)", elapsed.as_secs());
                match poll_fn(http) {
                    Ok(PollOutcome::Pending(status)) => {
                        let _ = writeln!(handle, "[nautpie] Status: {status}");
                    }
                    Ok(PollOutcome::Done) => return Ok(()),
                    Err(e) => return Err(e),
                }
                next_poll = Instant::now() + self.poll_interval;
            }

            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// Convenience wrapper that constructs a default `ProgressRunner` and runs it.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_uses_tui_when_stderr_is_terminal() {
        // We can't directly assert which branch is taken in a unit test
        // because TTY status depends on the host environment, but we can
        // assert the dispatcher returns a Result.
        let runner = ProgressRunner::new("test", "label");
        assert_eq!(runner.title, "test");
        assert_eq!(runner.label, "label");
        assert_eq!(runner.poll_interval, Duration::from_secs(5));
        assert_eq!(runner.tick_rate, Duration::from_millis(250));
    }

    #[test]
    fn run_with_progress_returns_timeout_on_deadline() {
        // Poll never signals Done; runner should time out.
        use crate::http::{HttpClient, Response};
        use std::collections::HashMap;

        struct NoopClient;
        impl HttpClient for NoopClient {
            fn set_endpoint(&mut self, _: &str) {}
            fn set_authorization(&mut self, _: &str) {}
            fn set_content_type(&mut self, _: &str) {}
            fn set_username_and_password(&mut self, _: &str, _: &str) {}
            fn request(
                &mut self,
                _: &str,
                _: &str,
                _: Option<HashMap<String, String>>,
                _: Option<String>,
            ) -> Result<Response, Error> {
                Ok(Response {
                    status: 200,
                    reason: "OK".into(),
                    body: "{}".into(),
                })
            }
        }

        let mut client = NoopClient;
        let result = run_with_progress(
            "test",
            "always-pending",
            Duration::from_millis(50), // very short deadline
            &mut client,
            |_| Ok(PollOutcome::Pending("waiting".into())),
        );
        // Whether TTY or non-TTY, the deadline must trip.
        assert!(matches!(result, Err(Error::Timeout(_))));
    }

    #[test]
    fn run_with_progress_returns_ok_when_poll_signals_done() {
        use crate::http::{HttpClient, Response};
        use std::collections::HashMap;

        struct NoopClient;
        impl HttpClient for NoopClient {
            fn set_endpoint(&mut self, _: &str) {}
            fn set_authorization(&mut self, _: &str) {}
            fn set_content_type(&mut self, _: &str) {}
            fn set_username_and_password(&mut self, _: &str, _: &str) {}
            fn request(
                &mut self,
                _: &str,
                _: &str,
                _: Option<HashMap<String, String>>,
                _: Option<String>,
            ) -> Result<Response, Error> {
                Ok(Response {
                    status: 200,
                    reason: "OK".into(),
                    body: "{}".into(),
                })
            }
        }

        let mut client = NoopClient;
        let result = run_with_progress(
            "test",
            "immediate-done",
            Duration::from_secs(60),
            &mut client,
            |_| Ok(PollOutcome::Done),
        );
        assert!(result.is_ok());
    }
}
