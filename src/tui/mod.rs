//! Terminal progress UI used by `gitFetch` and `checkDeploymentProgress`.
//!
//! In a TTY, a ratatui spinner renders the elapsed time and the most recent
//! status from the polled HTTP endpoint. In a non-TTY (CI), the spinner
//! falls back to a stderr "Waiting…" line every 5 seconds, matching PHP's
//! behaviour.

pub mod spinner;

pub use spinner::{run_with_progress, PollOutcome, ProgressRunner};
