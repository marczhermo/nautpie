//! Entry point: parse CLI → resolve subcommand + action → dispatch → print
//! JSON line on stdout → exit 0 on success / 1 on failure.
//!
//! Mirrors the PHP `nautpie.php` entrypoint. The dispatcher produces a single
//! JSON line per invocation; any non-zero exit is signaled through the
//! `status` field of the envelope.

use clap::Parser;
use nautpie::cli::{normalise_action, Cli, CommandKind};
use nautpie::commands::bitbucket::Bitbucket;
use nautpie::commands::deploy_naut::{
    options_from_bitbucket_args, options_from_deploy_args, DeployNaut,
};
use nautpie::commands::Command;
use nautpie::env::load_dotenv;
use nautpie::error::{ApiResponse, Error};
use nautpie::http::{HttpClient, ReqwestHttpClient};
use nautpie::io::{Io, StderrIo};

fn main() {
    // Step 1: try to load .env; failures are ignored (mirrors PHP safeLoad).
    let _ = load_dotenv();

    let cli = Cli::parse();
    let mut io = StderrIo::new();
    let mut http = ReqwestHttpClient::new().expect("reqwest client build");

    let exit_code = dispatch(cli, &mut io, &mut http);
    std::process::exit(exit_code);
}

fn dispatch(cli: Cli, io: &mut dyn Io, http: &mut dyn HttpClient) -> i32 {
    let action_raw = match &cli.command {
        CommandKind::DeployNaut(args) => args.action.clone(),
        CommandKind::Bitbucket(args) => args.action.clone(),
    };

    // Stash the parsed options bag on the Io impl so actions can read it.
    match &cli.command {
        CommandKind::DeployNaut(args) => io.set_options(options_from_deploy_args(args)),
        CommandKind::Bitbucket(args) => io.set_options(options_from_bitbucket_args(args)),
    }

    let result = match cli.command {
        CommandKind::DeployNaut(_) => {
            let cmd = DeployNaut;
            cmd.run(&normalise_action(&action_raw), io, http)
        }
        CommandKind::Bitbucket(_) => {
            let cmd = Bitbucket;
            cmd.run(&normalise_action(&action_raw), io, http)
        }
    };

    finish(result, io)
}

fn finish(result: Result<ApiResponse, Error>, io: &mut dyn Io) -> i32 {
    match result {
        Ok(response) => {
            io.write_json(&response);
            0
        }
        Err(err) => {
            // PHP parity: log the raw message on stderr, then emit a
            // JSON envelope with status=err-code (default 1) so downstream
            // CI parsers can decode a single line on stdout.
            io.warning(&err.to_string());
            let body = serde_json::Value::String(err.to_string());
            let response = ApiResponse::error(1, "Bad Request", body);
            io.write_json(&response);
            1
        }
    }
}
