//! CLI options bag used by command actions to read parsed clap values.
//!
//! Mirrors the PHP `getOptions()` accessor on `InputOutputHelper`.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Options {
    values: HashMap<String, String>,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    pub fn option(&self, name: &str) -> Option<String> {
        self.get(name)
    }
}
