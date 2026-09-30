//! Command implementations for the two CLI subcommands.
//!
//! - [`sample`] — the `sampleSuccess` / `sampleFail` smoke actions that
//!   exercise the dispatcher without making network calls.
//! - [`deploy_naut`] — DeployNaut API client (stubbed in chunk 1, filled in
//!   in chunk 8).
//! - [`bitbucket`] — Bitbucket Pipelines client (stubbed in chunk 1, filled
//!   in in chunk 9).

pub mod bitbucket;
pub mod deploy_naut;
pub mod sample;

use crate::error::{ApiResponse, Error};
use crate::http::HttpClient;
use crate::io::Io;

/// A CLI subcommand. Each implementation owns its action routing.
#[allow(dead_code)]
pub trait Command {
    /// Subcommand name, e.g. `"deploy:naut"` or `"ci:bitbucket"`.
    fn name(&self) -> &'static str;

    /// Run the requested (already case-normalised) action.
    fn run(
        &self,
        action: &str,
        io: &mut dyn Io,
        http: &mut dyn HttpClient,
    ) -> Result<ApiResponse, Error>;
}
