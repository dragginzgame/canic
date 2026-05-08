use crate::{
    args::{
        default_icp, local_network, parse_matches, print_help_or_version, string_option, value_arg,
    },
    version_text,
};
use canic_host::{
    icp::IcpCli,
    install_root::{InstallState, read_named_fleet_install_state},
    replica_query,
    table::WhitespaceTable,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs};
use thiserror::Error as ThisError;

const CHECK_HEADER: &str = "CHECK";
const STATUS_HEADER: &str = "STATUS";
const DETAIL_HEADER: &str = "DETAIL";
const NEXT_HEADER: &str = "NEXT";
const MEDIC_HELP_AFTER: &str = "\
Examples:
  canic medic test
  canic medic test --network local --icp icp";

///
/// MedicCommandError
///

#[derive(Debug, ThisError)]
pub enum MedicCommandError {
    #[error("{0}")]
    Usage(String),
}

///
/// MedicOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedicOptions {
    pub fleet: String,
    pub network: String,
    pub icp: String,
}

impl MedicOptions {
    /// Parse medic options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, MedicCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(medic_command(), args).map_err(|_| MedicCommandError::Usage(usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

/// Run read-only local Canic setup diagnostics.
pub fn run<I>(args: I) -> Result<(), MedicCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = MedicOptions::parse(args)?;
    println!("{}", render_medic_report(&run_medic_checks(&options)));
    Ok(())
}

// Build the medic parser and help metadata.
fn medic_command() -> ClapCommand {
    ClapCommand::new("medic")
        .bin_name("canic medic")
        .about("Diagnose local Canic fleet setup")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to inspect"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("ICP CLI network to inspect"),
        )
        .arg(
            value_arg("icp")
                .long("icp")
                .value_name("path")
                .help("Path to the icp executable"),
        )
        .after_help(MEDIC_HELP_AFTER)
}

// Run each diagnostic in dependency order so later checks can reuse fleet state.
fn run_medic_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = Vec::new();
    checks.push(MedicCheck::ok(
        "network",
        options.network.clone(),
        "override with --network <name>",
    ));
    checks.push(check_icp_cli(options));

    let state = match read_named_fleet_install_state(&options.network, &options.fleet) {
        Ok(Some(state)) => {
            checks.push(MedicCheck::ok(
                "fleet state",
                format!("{} installed", state.fleet),
                "run canic fleet list",
            ));
            Some(state)
        }
        Ok(None) => {
            checks.push(MedicCheck::warn(
                "fleet state",
                "no installed fleet found",
                "run canic install <name>",
            ));
            None
        }
        Err(err) => {
            checks.push(MedicCheck::error(
                "fleet state",
                err.to_string(),
                "reinstall from a config with [fleet].name",
            ));
            None
        }
    };

    if let Some(state) = state {
        checks.push(check_config_path(&state));
        checks.push(check_root_ready(options, &state));
    }

    checks
}

// Check whether the selected ICP CLI is available.
fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    match IcpCli::new(&options.icp, None, Some(options.network.clone())).version() {
        Ok(version) => MedicCheck::ok("icp cli", version, "-"),
        Err(err) => MedicCheck::error(
            "icp cli",
            err.to_string(),
            "install icp-cli or pass --icp <path>",
        ),
    }
}

// Check whether the saved install config still exists.
fn check_config_path(state: &InstallState) -> MedicCheck {
    if fs::metadata(&state.config_path).is_ok_and(|metadata| metadata.is_file()) {
        MedicCheck::ok("config", state.config_path.clone(), "-")
    } else {
        MedicCheck::error(
            "config",
            format!("missing {}", state.config_path),
            "restore the config or reinstall the fleet",
        )
    }
}

// Query the root readiness endpoint without mutating the canister.
fn check_root_ready(options: &MedicOptions, state: &InstallState) -> MedicCheck {
    let ready = if replica_query::should_use_local_replica_query(Some(&options.network)) {
        replica_query::query_ready(Some(&options.network), &state.root_canister_id)
            .map_err(|err| err.to_string())
    } else {
        query_ready_with_icp(options, &state.root_canister_id)
    };

    match ready {
        Ok(true) => MedicCheck::ok(
            "root ready",
            "canic_ready=true",
            format!("run canic list {}", options.fleet),
        ),
        Ok(false) => MedicCheck::warn(
            "root ready",
            "canic_ready=false",
            "wait briefly, then run canic medic",
        ),
        Err(err) => MedicCheck::error("root ready", err, "run canic install"),
    }
}

// Query readiness through ICP CLI for non-local networks.
fn query_ready_with_icp(options: &MedicOptions, canister: &str) -> Result<bool, String> {
    let output = IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .canister_call_output(canister, "canic_ready", Some("json"))
        .map_err(|err| err.to_string())?;
    let data = serde_json::from_str::<serde_json::Value>(&output).map_err(|err| err.to_string())?;
    Ok(replica_query::parse_ready_json_value(&data))
}

// Render medic checks as an operator-facing whitespace table.
fn render_medic_report(checks: &[MedicCheck]) -> String {
    let mut table = WhitespaceTable::new([CHECK_HEADER, STATUS_HEADER, DETAIL_HEADER, NEXT_HEADER]);
    for check in checks {
        table.push_row([
            check.name.as_str(),
            check.status.label(),
            check.detail.as_str(),
            check.next.as_str(),
        ]);
    }
    table.render()
}

// Return medic command help text.
fn usage() -> String {
    let mut command = medic_command();
    command.render_help().to_string()
}

///
/// MedicCheck
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct MedicCheck {
    name: String,
    status: MedicStatus,
    detail: String,
    next: String,
}

impl MedicCheck {
    // Build a successful diagnostic row.
    fn ok(name: impl Into<String>, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: MedicStatus::Ok,
            detail: detail.into(),
            next: next.into(),
        }
    }

    // Build a warning diagnostic row.
    fn warn(name: impl Into<String>, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: MedicStatus::Warn,
            detail: detail.into(),
            next: next.into(),
        }
    }

    // Build a failed diagnostic row.
    fn error(name: impl Into<String>, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: MedicStatus::Error,
            detail: detail.into(),
            next: next.into(),
        }
    }
}

///
/// MedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MedicStatus {
    Ok,
    Warn,
    Error,
}

impl MedicStatus {
    // Return the stable table label for one diagnostic status.
    const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure medic options parse the fleet, network, and ICP CLI selectors.
    #[test]
    fn parses_medic_options() {
        let options = MedicOptions::parse([
            OsString::from("demo"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--icp"),
            OsString::from("/tmp/icp"),
        ])
        .expect("parse medic options");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, "local");
        assert_eq!(options.icp, "/tmp/icp");
    }

    // Ensure medic help explains the diagnostic command rather than printing a one-liner.
    #[test]
    fn medic_usage_includes_examples() {
        let text = usage();

        assert!(text.contains("Diagnose local Canic fleet setup"));
        assert!(text.contains("Usage: canic medic [OPTIONS] <fleet>"));
        assert!(text.contains("<fleet>"));
        assert!(!text.contains("--fleet <name>"));
        assert!(text.contains("Examples:"));
    }

    // Ensure the medic report is a stable whitespace table.
    #[test]
    fn renders_medic_report() {
        let report = render_medic_report(&[
            MedicCheck::ok("network", "local", "-"),
            MedicCheck::warn(
                "fleet state",
                "no installed fleet found",
                "run canic install",
            ),
        ]);

        assert!(report.starts_with("CHECK"));
        assert!(report.contains("network"));
        assert!(report.contains("fleet state"));
        assert!(report.contains("warn"));
    }

    // Ensure common command-line JSON shapes are accepted for readiness.
    #[test]
    fn parses_ready_json_shapes() {
        assert!(replica_query::parse_ready_json_value(&serde_json::json!(
            true
        )));
        assert!(replica_query::parse_ready_json_value(
            &serde_json::json!([{"Ok": true}])
        ));
        assert!(!replica_query::parse_ready_json_value(
            &serde_json::json!([{"Ok": false}])
        ));
    }
}
