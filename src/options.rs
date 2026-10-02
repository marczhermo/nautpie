//! CLI options bag used by command actions to read parsed clap values.
//!
//! Mirrors the PHP `getOptions()` accessor on `InputOutputHelper`.

use std::collections::HashMap;

/// String-keyed bag of parsed CLI options.
///
/// Wraps a `HashMap<String, String>` so the storage representation can
/// change (e.g., to `BTreeMap` for deterministic ordering) without
/// affecting any caller.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The underlying key-value store. Kept private; use [`Options::set`],
    /// [`Options::get`], or [`Options::option`] instead.
    values: HashMap<String, String>,
}

impl Options {
    /// Create a new, empty [`Options`] bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a key-value pair.
    ///
    /// Accepts anything convertible into `String` for both key and value.
    ///
    /// # Arguments
    ///
    /// * `key` - The option name (e.g. `"stack"`).
    /// * `value` - The option value (e.g. `"example"`).
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Look up an option by name. Returns `None` if the key was never set.
    ///
    /// # Arguments
    ///
    /// * `key` - The option name to look up.
    pub fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    /// Alias for [`Options::get`] that mirrors the PHP `getOptions()` helper.
    ///
    /// # Arguments
    ///
    /// * `name` - The option name to look up.
    pub fn option(&self, name: &str) -> Option<String> {
        self.get(name)
    }
}
