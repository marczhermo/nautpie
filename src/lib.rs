//! NautPie — DeployNaut / Bitbucket Pipelines deployment client.
//!
//! Rust port of the PHP `nautpie.phar` (Symfony Console). This crate exposes the
//! CLI dispatch surface and supporting helpers; the binary entrypoint is
//! [`main`] in `src/main.rs`.

pub mod boolean;
pub mod cli;
pub mod commands;
pub mod deployment_details;
pub mod env;
pub mod error;
pub mod http;
pub mod io;
pub mod options;
pub mod tui;

pub use error::{ApiResponse, Error};
