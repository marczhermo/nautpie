//! Plain stderr/stdout sink. Uses `owo-colors` to give warnings / success
//! messages a coloured badge, matching the PHP version's
//! `<fg=red;bg=yellow>` / `<fg=black;bg=green>` styling.

use std::io::{self, Write};

use owo_colors::{OwoColorize, Stream};

use super::Io;
use crate::error::ApiResponse;
use crate::options::Options;

pub struct StderrIo {
    opts: Option<Options>,
}

impl StderrIo {
    pub fn new() -> Self {
        Self { opts: None }
    }
}

impl Default for StderrIo {
    fn default() -> Self {
        Self::new()
    }
}

impl Io for StderrIo {
    fn warning(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "  {}  {}  ",
            "WARN".if_supports_color(Stream::Stderr, |t| t.on_yellow().red()),
            message
        );
    }

    fn success(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "  {}  {}  ",
            " OK ".if_supports_color(Stream::Stderr, |t| t.on_green().black()),
            message
        );
    }

    fn message(&mut self, message: &str) {
        let mut stderr = io::stderr().lock();
        let _ = writeln!(stderr, "  {}  ", message);
    }

    fn write_json(&mut self, response: &ApiResponse) {
        println!("{}", response.to_line());
    }

    fn set_options(&mut self, opts: Options) {
        self.opts = Some(opts);
    }

    fn options(&self) -> Option<Options> {
        self.opts.clone()
    }
}
