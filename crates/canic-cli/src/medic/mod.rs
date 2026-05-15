use crate::{
    cli::clap::{parse_matches, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    install_root::InstallState,
    installed_fleet::read_installed_fleet_state_from_root,
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::Path};
use thiserror::Error as ThisError;

const CHECK_HEADER: &str = "CHECK";
const STATUS_HEADER: &str = "STATUS";
const DETAIL_HEADER: &str = "DETAIL";
const NEXT_HEADER: &str = "NEXT";
const MEDIC_HELP_AFTER: &str = "\
Examples:
  canic medic test";

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
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .after_help(MEDIC_HELP_AFTER)
}

fn run_medic_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = Vec::new();
    let icp_root = resolve_current_canic_icp_root().ok();
    checks.push(MedicCheck::ok(
        "network",
        options.network.clone(),
        "override with top-level --network <name>",
    ));
    checks.push(check_icp_cli(options));

    let state = match icp_root.as_deref().map_or_else(
        || Err("could not resolve ICP project root".to_string()),
        |root| {
            read_installed_fleet_state_from_root(&options.network, &options.fleet, root)
                .map_err(|err| err.to_string())
        },
    ) {
        Ok(state) => {
            checks.push(MedicCheck::ok(
                "fleet state",
                format!("{} installed", state.fleet),
                "run canic fleet list",
            ));
            Some(state)
        }
        Err(err) if is_missing_installed_fleet(&err) => {
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
                err,
                "reinstall from a config with [fleet].name",
            ));
            None
        }
    };

    if let Some(state) = state {
        checks.push(check_config_path(&state));
        checks.push(check_root_ready(options, icp_root.as_deref(), &state));
    }

    checks
}

fn is_missing_installed_fleet(error: &str) -> bool {
    error.starts_with("fleet ") && error.contains(" is not installed on network ")
}

fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    match IcpCli::new(&options.icp, None, Some(options.network.clone())).version() {
        Ok(version) => MedicCheck::ok("icp cli", version, "-"),
        Err(err) => MedicCheck::error(
            "icp cli",
            err.to_string(),
            "install icp-cli or pass top-level --icp <path>",
        ),
    }
}

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

fn check_root_ready(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    state: &InstallState,
) -> MedicCheck {
    let ready = if replica_query::should_use_local_replica_query(Some(&options.network)) {
        icp_root
            .map_or_else(
                || replica_query::query_ready(Some(&options.network), &state.root_canister_id),
                |root| {
                    replica_query::query_ready_from_root(
                        Some(&options.network),
                        &state.root_canister_id,
                        root,
                    )
                },
            )
            .map_err(|err| err.to_string())
    } else {
        query_ready_with_icp(options, icp_root, &state.root_canister_id)
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

fn query_ready_with_icp(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    canister: &str,
) -> Result<bool, String> {
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    let output = icp
        .canister_query_output(canister, "canic_ready", Some("json"))
        .map_err(|err| err.to_string())?;
    let data = serde_json::from_str::<serde_json::Value>(&output).map_err(|err| err.to_string())?;
    Ok(replica_query::parse_ready_json_value(&data))
}

fn render_medic_report(checks: &[MedicCheck]) -> String {
    let rows = checks
        .iter()
        .map(|check| {
            [
                check.name.clone(),
                check.status.label().to_string(),
                check.detail.clone(),
                check.next.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &[CHECK_HEADER, STATUS_HEADER, DETAIL_HEADER, NEXT_HEADER],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

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
    fn ok(name: impl Into<String>, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: MedicStatus::Ok,
            detail: detail.into(),
            next: next.into(),
        }
    }

    fn warn(name: impl Into<String>, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: MedicStatus::Warn,
            detail: detail.into(),
            next: next.into(),
        }
    }

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
    const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests;
