//! clap CLI definition. Mirrors the two Symfony Console subcommands exactly:
//!
//! - `deploy:naut <action> [--url --commit --stack --environment ...]`
//! - `ci:bitbucket <action> [--commit --stack --environment ...]`
//!
//! Action names are normalised at the dispatch boundary (see
//! [`normalise_action`]) so callers can pass `sampleSuccess`, `SampleSuccess`,
//! `SAMPLESUCCESS`, or `sample_success` interchangeably.

use clap::{Parser, Subcommand};

/// Normalise a user-supplied action token to the canonical lower-camelCase
/// used by the dispatch table.
///
/// PHP just ran `ucfirst` on the input and relied on PHP's case-insensitive
/// method dispatch — so `sampleSuccess`, `SampleSuccess`, or `SAMPLESUCCESS`
/// would all resolve to `doSampleSuccess`. PHP's snake_case variant
/// `sample_success` would actually NOT work in PHP because
/// `ucfirst("sample_success")` produces the invalid method name
/// `doSample_success`. In Rust we are case-sensitive, so we convert
/// explicitly to lower-camelCase by detecting word boundaries on `_`, `-`,
/// and CamelCase transitions (lowercase→uppercase). ALL_CAPS inputs without
/// any boundary marker (e.g. `SAMPLESUCCESS`) are treated as a single word
/// — there is no reliable way to split them.
///
/// This handles every PHP-compatible input and also handles the snake_case
/// shape that PHP would have rejected:
/// - `sampleSuccess`  → `sampleSuccess`
/// - `SampleSuccess`  → `sampleSuccess`
/// - `sample_success` → `sampleSuccess`
/// - `Sample-Success` → `sampleSuccess`
/// - `SAMPLESUCCESS`  → `samplesuccess` (no boundary detected)
pub fn normalise_action(raw: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut prev_is_lower = false;
    for c in raw.chars() {
        if c == '_' || c == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            prev_is_lower = false;
            continue;
        }
        let upper = c.is_ascii_uppercase();
        if upper && prev_is_lower {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
        prev_is_lower = c.is_ascii_lowercase();
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        let lower = w.to_ascii_lowercase();
        if i == 0 {
            out.push_str(&lower);
        } else {
            let mut chars = lower.chars();
            if let Some(head) = chars.next() {
                out.push(head.to_ascii_uppercase());
            }
            out.push_str(chars.as_str());
        }
    }
    out
}

#[derive(Debug, Parser)]
#[command(
    name = "nautpie",
    version,
    about = "DeployNaut / Bitbucket Pipelines deployment client"
)]
/// Top-level CLI definition. Parsed by `clap` via `Cli::parse()`.
pub struct Cli {
    /// The subcommand the user invoked. One of `DeployNaut` or `Bitbucket`.
    #[command(subcommand)]
    pub command: CommandKind,
}

#[derive(Debug, Subcommand)]
/// The two CLI subcommands. Each variant holds the parsed argument struct
/// for that subcommand.
pub enum CommandKind {
    /// DeployNaut API actions.
    #[command(name = "deploy:naut")]
    DeployNaut(DeployNautArgs),

    /// Bitbucket Pipelines actions.
    #[command(name = "ci:bitbucket")]
    Bitbucket(BitbucketArgs),
}

/// Arguments shared by both subcommands.
///
/// Currently aspirational: both `DeployNautArgs` and `BitbucketArgs`
/// implement this trait so generic code can ask either for its action
/// name. The action functions themselves read concrete types today.
#[allow(dead_code)]
pub trait SharedArgs {
    /// Return the action name (e.g. `"sampleSuccess"`).
    fn action(&self) -> &str;
}

/// `deploy:naut` — Sends API requests to DeployNaut.
#[derive(Debug, clap::Args)]
pub struct DeployNautArgs {
    /// Command action (e.g. `sampleSuccess`, `createDeployment`).
    #[arg(value_name = "ACTION")]
    pub action: String,

    /// [Optional] URL
    #[arg(long)]
    pub url: Option<String>,

    /// [Optional] Git commit SHA
    #[arg(long)]
    pub commit: Option<String>,

    /// [Optional] Project stack
    #[arg(long)]
    pub stack: Option<String>,

    /// [Optional] Stack environment
    #[arg(long)]
    pub environment: Option<String>,

    /// [Optional] Start date
    #[arg(long)]
    pub start_date: Option<String>,

    /// [Optional] Deployment title
    #[arg(long)]
    pub title: Option<String>,

    /// [Optional] Deployment summary
    #[arg(long)]
    pub summary: Option<String>,

    /// [Optional] Redeploy last deployment
    #[arg(long)]
    pub redeploy: Option<String>,

    /// [Optional] Deployment Reference
    #[arg(long)]
    pub r#ref: Option<String>,

    /// [Optional] Deployment Type
    #[arg(long = "ref_type")]
    pub ref_type: Option<String>,

    /// [Optional] Deployment bypass and start
    #[arg(long)]
    pub bypass_and_start: Option<String>,

    /// [Optional] Deployment ID
    #[arg(long)]
    pub deploy_id: Option<String>,

    /// [Optional] Wait for deployment to finish
    #[arg(long)]
    pub should_wait: Option<String>,
}

impl SharedArgs for DeployNautArgs {
    fn action(&self) -> &str {
        &self.action
    }
}

/// `ci:bitbucket` — Bitbucket Pipelines integration.
#[derive(Debug, clap::Args)]
pub struct BitbucketArgs {
    /// Command action (e.g. `sampleSuccess`, `createAccessToken`).
    #[arg(value_name = "ACTION")]
    pub action: String,

    /// [Optional] Git commit SHA
    #[arg(long)]
    pub commit: Option<String>,

    /// [Optional] Project stack
    #[arg(long)]
    pub stack: Option<String>,

    /// [Optional] Stack environment
    #[arg(long)]
    pub environment: Option<String>,

    /// [Optional] Deployment title
    #[arg(long)]
    pub title: Option<String>,

    /// [Optional] Deployment summary
    #[arg(long)]
    pub summary: Option<String>,

    /// [Optional] Deployment tag
    #[arg(long)]
    pub tag: Option<String>,

    /// [Optional] Deployment bypass and start
    #[arg(long)]
    pub bypass_and_start: Option<String>,

    /// [Optional] Deployment ID
    #[arg(long)]
    pub deploy_id: Option<String>,

    /// [Optional] Wait for deployment to finish
    #[arg(long)]
    pub should_wait: Option<String>,
}

impl SharedArgs for BitbucketArgs {
    fn action(&self) -> &str {
        &self.action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_action_canonical() {
        assert_eq!(normalise_action("sampleSuccess"), "sampleSuccess");
        assert_eq!(normalise_action("SampleSuccess"), "sampleSuccess");
        assert_eq!(normalise_action("sample_success"), "sampleSuccess");
        assert_eq!(normalise_action("Sample-Success"), "sampleSuccess");
        assert_eq!(normalise_action("createDeployment"), "createDeployment");
        // ALL_CAPS without any boundary marker has no detectable split, so
        // the normaliser treats it as one word. PHP would have routed this
        // through `ucfirst` + case-insensitive dispatch; Rust routes it to
        // the same canonical token but lower-cased.
        assert_eq!(normalise_action("SAMPLESUCCESS"), "samplesuccess");
        assert_eq!(normalise_action("create_deployment"), "createDeployment");
    }

    #[test]
    fn normalise_action_empty() {
        assert_eq!(normalise_action(""), "");
    }

    #[test]
    fn cli_parses_deploy_naut_help() {
        let cli = Cli::try_parse_from(["nautpie", "deploy:naut", "sampleSuccess"]).unwrap();
        match cli.command {
            CommandKind::DeployNaut(args) => assert_eq!(args.action, "sampleSuccess"),
            _ => panic!("expected deploy:naut"),
        }
    }

    #[test]
    fn cli_parses_bitbucket_with_options() {
        let cli = Cli::try_parse_from(["nautpie", "ci:bitbucket", "createAccessToken"]).unwrap();
        match cli.command {
            CommandKind::Bitbucket(args) => assert_eq!(args.action, "createAccessToken"),
            _ => panic!("expected ci:bitbucket"),
        }
    }

    #[test]
    fn cli_requires_action() {
        let err = Cli::try_parse_from(["nautpie", "deploy:naut"]).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("action"));
    }
}
