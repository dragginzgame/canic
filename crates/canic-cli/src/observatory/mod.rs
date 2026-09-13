//! Operator snapshot and curated downstream report adapter.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_nested_help,
    },
    support::icp_target::IcpTargetOptions,
};
use canic_host::{
    icp_config::resolve_current_canic_icp_root,
    observatory::{
        model::{ObservatoryOptions, ObservatoryProfile},
        ops::{self, presentation},
        workflow,
    },
};
use clap::Command;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
};

/// Adapter errors preserve host diagnostics and local publication failures.
#[derive(Debug, thiserror::Error)]
pub enum ObservatoryCommandError {
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    Host(#[from] canic_host::observatory::ObservatoryError),
    #[error(transparent)]
    Root(#[from] canic_host::icp_config::IcpConfigError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn command() -> Command {
    Command::new("observatory").bin_name("canic observatory")
        .about("Collect bounded role observations and render a curated public report")
        .arg(internal_environment_arg().global(true)).arg(internal_icp_arg().global(true))
        .subcommand_required(true)
        .subcommand(Command::new("snapshot").about("Query one terminal Fleet once; preserve independent failures")
            .arg(value_arg("fleet").required(true))
            .arg(value_arg("freshness-secs").long("freshness-secs").default_value("30").value_parser(clap::value_parser!(u32).range(1..=3600)))
            .arg(value_arg("maximum-collection-secs").long("maximum-collection-secs").default_value("60").value_parser(clap::value_parser!(u32).range(1..=3600)))
            .arg(value_arg("maximum-canisters").long("maximum-canisters").default_value("128").value_parser(clap::value_parser!(u32).range(1..=4096)))
            .arg(value_arg("maximum-response-bytes").long("maximum-response-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("maximum-report-bytes").long("maximum-report-bytes").default_value("1048576").value_parser(clap::value_parser!(u32).range(1024..=16_777_216)))
            .arg(value_arg("query-timeout-secs").long("query-timeout-secs").default_value("5").value_parser(clap::value_parser!(u32).range(1..=60)))
            .arg(value_arg("profile").long("profile").help("Data-only public labels JSON; required for --public"))
            .arg(flag_arg("public").long("public").requires("profile").help("Omit exact authority, identities, placement and funding"))
            .arg(flag_arg("html").long("html").requires("public").help("Render escaped public HTML instead of JSON"))
            .arg(value_arg("out").long("out").help("Create a new report file; otherwise write stdout")))
        .after_help("Examples:\n  canic observatory snapshot demo\n  canic observatory snapshot demo --public --profile observatory.json --html --out report.html")
}

/// Collect once and publish only the explicitly selected report surface.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), ObservatoryCommandError> {
    let args = args.into_iter().collect::<Vec<_>>();
    if print_nested_help(&args, command()) {
        return Ok(());
    }
    let matches = parse_matches(command(), args)?;
    let (_, leaf) = matches.subcommand().expect("required snapshot command");
    let target = IcpTargetOptions::parse(&matches);
    let options = ObservatoryOptions {
        environment: target.environment.clone(),
        fleet: required_string(leaf, "fleet"),
        freshness_secs: *leaf.get_one::<u32>("freshness-secs").unwrap(),
        maximum_canisters: *leaf.get_one::<u32>("maximum-canisters").unwrap() as usize,
        maximum_response_bytes: *leaf.get_one::<u32>("maximum-response-bytes").unwrap() as usize,
        maximum_collection_secs: *leaf.get_one::<u32>("maximum-collection-secs").unwrap(),
        query_timeout_secs: *leaf.get_one::<u32>("query-timeout-secs").unwrap(),
    };
    let maximum = *leaf.get_one::<u32>("maximum-report-bytes").unwrap() as usize;
    // Validate the profile before any remote observation.
    let profile = leaf
        .get_one::<String>("profile")
        .map(|path| read_profile(path))
        .transpose()?;
    let root = resolve_current_canic_icp_root()?;
    let snapshot = workflow::snapshot(&root, &target.icp_cli(&root), &options)?;
    let bytes = if leaf.get_flag("public") {
        presentation::http_response(
            &snapshot,
            profile.as_ref().expect("required public profile"),
            if leaf.get_flag("html") {
                "/"
            } else {
                "/snapshot.json"
            },
            ops::now_ms(),
            maximum,
        )?
        .body
    } else {
        presentation::json_bytes(&snapshot, maximum)?
    };
    if let Some(path) = leaf.get_one::<String>("out") {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    } else {
        std::io::stdout().lock().write_all(&bytes)?;
        println!();
    }
    Ok(())
}

fn read_profile(path: &str) -> Result<ObservatoryProfile, ObservatoryCommandError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(canic_host::observatory::ObservatoryError::Bound("profile bytes").into());
    }
    let profile = serde_json::from_slice(&bytes)?;
    canic_host::observatory::policy::validate_profile(&profile)?;
    Ok(profile)
}
