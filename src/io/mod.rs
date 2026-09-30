//! Output sink abstraction.
//!
//! The CLI writes user-facing status messages (warnings, success ticks,
//! info) to **stderr** to keep stdout reserved for the single JSON-line
//! response envelope. The Rust port keeps that contract.

pub mod stderr_io;

pub use stderr_io::StderrIo;

use crate::error::ApiResponse;
use crate::options::Options;

/// A sink for the binary's progress messages and final JSON-line response.
pub trait Io {
    /// Print a yellow-on-red warning.
    fn warning(&mut self, message: &str);

    /// Print a black-on-green success line.
    fn success(&mut self, message: &str);

    /// Print a neutral info line.
    fn message(&mut self, message: &str);

    /// Print the final JSON-line response to stdout.
    fn write_json(&mut self, response: &ApiResponse);

    /// Inject the parsed CLI options bag so actions can read `--ref`,
    /// `--stack`, etc. The default implementation is a no-op for Io impls
    /// that do not need it.
    fn set_options(&mut self, _opts: Options) {}

    /// Retrieve the options bag that was injected via `set_options`.
    fn options(&self) -> Option<Options> {
        None
    }
}
