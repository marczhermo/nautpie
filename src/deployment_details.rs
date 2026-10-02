//! Fluent builder for the DeployNaut deployment payload. Mirrors the PHP
//! `DeploymentDetails` class.
//!
//! Note: the PHP `summary()` method had a typo (`summery`) that meant the
//! title was never actually persisted to the deployment payload. The Rust
//! port corrects this so calls like `details.summary("Branch:foo")` produce
//! a `summary` field in `values()`. Fixture data already uses the correct
//! `summary` key, so observable behaviour matches expectations.
//!
//! `values()` returns a `serde_json::Value` mirroring the PHP
//! `array_filter($details, 'isNotNull')` — null and empty-string fields are
//! stripped before serialisation.

use chrono::DateTime;
use serde_json::{Map, Value};

const DEFAULT_TITLE: &str = "[CI] Deployment";

/// In-memory representation of the deployment payload that will be POSTed to
/// DeployNaut's `project/{stack}/environment/{env}/deploys` endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentDetails {
    /// Git reference being deployed (commit SHA, branch name, or tag).
    /// Named `r#ref` in Rust because `ref` is a reserved keyword; serialised
    /// as `"ref"` in JSON.
    pub r#ref: Option<String>,
    /// Kind of reference: `"sha"`, `"branch"`, `"package"`, `"redeploy"`, or
    /// `"promote_from_uat"`.
    pub ref_type: String,
    /// Human-readable title shown in the DeployNaut dashboard.
    pub title: Option<String>,
    /// One-line summary, often the source branch name.
    pub summary: Option<String>,
    /// Skip approval workflow if truthy.
    pub bypass: bool,
    /// Skip approval **and** start the deployment immediately.
    pub bypass_and_start: bool,
    /// Optional Unix epoch (seconds) at which to start the deployment.
    pub schedule_start_unix: Option<i64>,
    /// Optional Unix epoch (seconds) at which the deployment must end.
    pub schedule_end_unix: Option<i64>,
    /// If truthy, lock the deployment so it cannot be cancelled.
    pub locked: bool,
}

impl Default for DeploymentDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl DeploymentDetails {
    /// New `DeploymentDetails` populated with PHP defaults.
    pub fn new() -> Self {
        Self {
            r#ref: Some(String::new()),
            ref_type: "sha".into(),
            title: Some(DEFAULT_TITLE.into()),
            summary: Some(String::new()),
            bypass: false,
            bypass_and_start: false,
            schedule_start_unix: None,
            schedule_end_unix: None,
            locked: false,
        }
    }

    /// Set `ref_type` (e.g. `"sha"`, `"branch"`, `"package"`).
    pub fn ref_type(mut self, value: impl Into<String>) -> Self {
        self.ref_type = value.into();
        self
    }

    /// Set the Git reference being deployed.
    pub fn ref_(mut self, value: impl Into<String>) -> Self {
        self.r#ref = Some(value.into());
        self
    }

    /// Set the deployment title shown in the dashboard.
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    /// Set the one-line summary (often the source branch name).
    pub fn summary(mut self, value: impl Into<String>) -> Self {
        self.summary = Some(value.into());
        self
    }

    /// Set `bypass_and_start` (skip approval and start immediately).
    pub fn bypass_and_start(mut self, yes: bool) -> Self {
        self.bypass_and_start = yes;
        self
    }

    /// Set the schedule_start_unix field by parsing a free-form time string.
    /// Accepts Unix timestamps (numeric strings) or RFC3339 timestamps.
    /// Mirrors PHP `strtotime`.
    pub fn schedule_to_start(mut self, time_str: impl AsRef<str>) -> Self {
        let raw = time_str.as_ref();
        self.schedule_start_unix = parse_unix(raw);
        self
    }

    /// Set `schedule_end_unix` by parsing a free-form time string.
    /// Accepts Unix timestamps or RFC3339.
    pub fn schedule_end(mut self, time_str: impl AsRef<str>) -> Self {
        let raw = time_str.as_ref();
        self.schedule_end_unix = parse_unix(raw);
        self
    }

    /// Mirror of `DeploymentDetails::redeploy($yes)` — when truthy, blank out
    /// `ref` and switch `ref_type` to `"redeploy"`.
    pub fn redeploy(mut self, yes: bool) -> Self {
        if yes {
            self.r#ref = Some(String::new());
            self.ref_type = "redeploy".into();
        }
        self
    }

    /// Mark this deployment as a UAT promotion. Clears `ref` and sets
    /// `ref_type` to `"promote_from_uat"`.
    pub fn promote_from_uat(mut self) -> Self {
        self.r#ref = Some(String::new());
        self.ref_type = "promote_from_uat".into();
        self
    }

    /// Lock the deployment so it cannot be cancelled.
    pub fn locked(mut self, yes: bool) -> Self {
        self.locked = yes;
        self
    }

    /// Render the deployment payload, stripping null and empty-string fields
    /// (mirrors PHP `array_filter($details, 'isNotNull')`).
    pub fn values(&self) -> Value {
        let mut map = Map::new();
        let mut insert = |key: &str, val: Value| {
            if !matches!(val, Value::Null) {
                map.insert(key.into(), val);
            }
        };

        // `ref` — strip empty string (matches PHP: empty string is falsy).
        match &self.r#ref {
            Some(s) if !s.is_empty() => insert("ref", Value::String(s.clone())),
            _ => {} // dropped
        }

        insert("ref_type", Value::String(self.ref_type.clone()));
        insert(
            "title",
            match &self.title {
                Some(s) if !s.is_empty() => Value::String(s.clone()),
                _ => Value::Null,
            },
        );
        insert(
            "summary",
            match &self.summary {
                Some(s) if !s.is_empty() => Value::String(s.clone()),
                _ => Value::Null,
            },
        );
        insert("bypass", Value::Bool(self.bypass));
        insert("bypass_and_start", Value::Bool(self.bypass_and_start));
        insert(
            "schedule_start_unix",
            match self.schedule_start_unix {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            },
        );
        insert(
            "schedule_end_unix",
            match self.schedule_end_unix {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            },
        );
        insert("locked", Value::Bool(self.locked));

        Value::Object(map)
    }
}

/// Parse a Unix timestamp (numeric string) or RFC3339 timestamp into i64
/// seconds. Mirrors PHP `strtotime` for the subset of formats used by the
/// tests (Unix epoch, ISO8601). PHP `strtotime("-1 year")` is not supported
/// in this subset — callers using relative strings must pre-compute the
/// epoch.
fn parse_unix(raw: &str) -> Option<i64> {
    if let Ok(n) = raw.parse::<i64>() {
        return Some(n);
    }
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_php_defaults() {
        let d = DeploymentDetails::new();
        assert_eq!(d.ref_type, "sha");
        assert_eq!(d.r#ref.as_deref(), Some(""));
        assert_eq!(d.title.as_deref(), Some("[CI] Deployment"));
        assert_eq!(d.summary.as_deref(), Some(""));
        assert!(!d.bypass);
        assert!(!d.bypass_and_start);
        assert_eq!(d.schedule_start_unix, None);
        assert_eq!(d.schedule_end_unix, None);
        assert!(!d.locked);
    }

    #[test]
    fn builder_chain_sets_fields() {
        let d = DeploymentDetails::new()
            .ref_("abc123")
            .ref_type("branch")
            .title("[CD] v1.2.3")
            .summary("Branch:main")
            .bypass_and_start(true)
            .locked(false);

        assert_eq!(d.r#ref.as_deref(), Some("abc123"));
        assert_eq!(d.ref_type, "branch");
        assert_eq!(d.title.as_deref(), Some("[CD] v1.2.3"));
        assert_eq!(d.summary.as_deref(), Some("Branch:main"));
        assert!(d.bypass_and_start);
        assert!(!d.locked);
    }

    #[test]
    fn schedule_to_start_parses_rfc3339() {
        let d = DeploymentDetails::new().schedule_to_start("2026-12-25T10:00:00Z");
        assert_eq!(d.schedule_start_unix, Some(1_798_192_800));
    }

    #[test]
    fn schedule_to_start_parses_unix_epoch() {
        let d = DeploymentDetails::new().schedule_to_start("1700000000");
        assert_eq!(d.schedule_start_unix, Some(1_700_000_000));
    }

    #[test]
    fn redeploy_truthy_clears_ref_and_switches_type() {
        let d = DeploymentDetails::new().ref_("abc").redeploy(true);
        assert_eq!(d.r#ref.as_deref(), Some(""));
        assert_eq!(d.ref_type, "redeploy");
    }

    #[test]
    fn redeploy_false_keeps_state() {
        let d = DeploymentDetails::new().ref_("abc").redeploy(false);
        assert_eq!(d.r#ref.as_deref(), Some("abc"));
        assert_eq!(d.ref_type, "sha");
    }

    #[test]
    fn promote_from_uat_clears_ref() {
        let d = DeploymentDetails::new().ref_("abc").promote_from_uat();
        assert_eq!(d.r#ref.as_deref(), Some(""));
        assert_eq!(d.ref_type, "promote_from_uat");
    }

    #[test]
    fn values_strips_null_and_empty() {
        let d = DeploymentDetails::new();
        let v = d.values();
        let obj = v.as_object().unwrap();
        // Empty `ref` and empty `summary` are stripped; the default title
        // "[CI] Deployment" is non-empty so it survives (mirrors PHP).
        assert!(!obj.contains_key("ref"));
        assert!(obj.contains_key("title"));
        assert!(!obj.contains_key("summary"));
        assert!(!obj.contains_key("schedule_start_unix"));
        assert!(!obj.contains_key("schedule_end_unix"));
        assert_eq!(obj["ref_type"], "sha");
        assert_eq!(obj["bypass"], false);
        assert_eq!(obj["bypass_and_start"], false);
        assert_eq!(obj["locked"], false);
    }

    #[test]
    fn values_keeps_default_title_but_drops_empty_ref_and_summary() {
        let d = DeploymentDetails::new();
        let v = d.values();
        let obj = v.as_object().unwrap();
        // The default title "[CI] Deployment" is non-empty, so it survives.
        assert_eq!(obj["title"], "[CI] Deployment");
        // Empty ref and empty summary are stripped.
        assert!(!obj.contains_key("ref"));
        assert!(!obj.contains_key("summary"));
    }

    #[test]
    fn values_includes_set_fields() {
        let d = DeploymentDetails::new()
            .ref_("abc123def456abc123def456abc123def45678")
            .ref_type("sha")
            .summary("Branch:main");
        let v = d.values();
        assert_eq!(v["ref"], "abc123def456abc123def456abc123def45678");
        assert_eq!(v["ref_type"], "sha");
        assert_eq!(v["summary"], "Branch:main");
    }
}
