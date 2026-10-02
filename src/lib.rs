//! NautPie — DeployNaut / Bitbucket Pipelines deployment client.
//!
//! Rust port of the PHP `nautpie.phar` (Symfony Console). This crate exposes the
//! CLI dispatch surface and supporting helpers; the binary entrypoint is
//! [`main`] in `src/main.rs`.
//!
//! # Crate overview
//!
//! The README is the canonical user-facing introduction. It is included below
//! (via `#![doc = include_str!("../README.md")]`) so that `cargo doc` renders
//! the same content that GitHub shows.
//!
//! # Module map
//!
//! | Module | What it does |
//! |---|---|
//! | [`boolean`] | Parse CLI string flags into `bool` (PHP parity). |
//! | [`cli`] | `clap` subcommand and action definitions. |
//! | [`commands`] | Action implementations (`deploy:naut`, `ci:bitbucket`). |
//! | [`deployment_details`] | Builder for the DeployNaut payload. |
//! | [`env`] | `.env` loading and required-variable checks. |
//! | [`error`] | Top-level [`Error`] enum and [`ApiResponse`] envelope. |
//! | [`http`] | HTTP client trait and `reqwest` implementation. |
//! | [`io`] | Output sink trait and stderr implementation. |
//! | [`options`] | String-keyed CLI options bag. |
//! | [`tui`] | TTY-aware progress spinner used by polling actions. |
//!
//! # Quickstart
//!
//! The binary is the primary interface; the library surface below is exposed
//! for embedding in other Rust programs. Example:
//!
//! ```no_run
//! use nautpie::error::ApiResponse;
//! use serde_json::json;
//!
//! let resp = ApiResponse::ok(json!("hello"));
//! assert_eq!(resp.status, 200);
//! ```

#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

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
