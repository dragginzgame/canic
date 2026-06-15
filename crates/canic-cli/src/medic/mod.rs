use crate::{
    cli::clap::{parse_matches, render_usage, required_string, string_option_or_else, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    support::candid::role_candid_path,
    version_text,
};
use canic_host::{
    canister_ready::query_canister_ready,
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    install_root::InstallState,
    installed_deployment::read_installed_deployment_state_from_root,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::Path};
use thiserror::Error as ThisError;

const CHECK_HEADER: &str = "CHECK";
const STATUS_HEADER: &str = "STATUS";
const DETAIL_HEADER: &str = "DETAIL";
const NEXT_HEADER: &str = "NEXT";
const INFO_MEDIC_HELP_AFTER: &str = "\
Examples:
  canic info medic test";

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
struct MedicOptions {
    deployment: String,
    network: String,
    icp: String,
}

impl MedicOptions {
    fn parse_info<I>(args: I) -> Result<Self, MedicCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, info_medic_command, info_usage)
    }

    fn parse_with<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, MedicCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| MedicCommandError::Usage(usage()))?;

        Ok(Self {
            deployment: required_string(&matches, "deployment"),
            network: string_option_or_else(&matches, "network", local_network),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

pub fn run_info<I>(args: I) -> Result<(), MedicCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }

    let options = MedicOptions::parse_info(args)?;
    run_options(&options);
    Ok(())
}

fn run_options(options: &MedicOptions) {
    println!("{}", render_medic_report(&run_medic_checks(options)));
}

fn info_medic_command() -> ClapCommand {
    medic_command_with_bin_name("canic info medic", INFO_MEDIC_HELP_AFTER)
}

fn medic_command_with_bin_name(bin_name: &'static str, help_after: &'static str) -> ClapCommand {
    ClapCommand::new("medic")
        .bin_name(bin_name)
        .about("Diagnose local Canic deployment target setup")
        .disable_help_flag(true)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Installed deployment name to inspect"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .after_help(help_after)
}

fn info_usage() -> String {
    render_usage(info_medic_command)
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
            read_installed_deployment_state_from_root(&options.network, &options.deployment, root)
                .map_err(|err| err.to_string())
        },
    ) {
        Ok(state) => {
            checks.push(MedicCheck::ok(
                "deployment state",
                format!("{} installed", state.deployment_name),
                "run canic fleet list",
            ));
            Some(state)
        }
        Err(err) if is_missing_installed_deployment(&err) => {
            checks.push(MedicCheck::warn(
                "deployment state",
                "no installed deployment found",
                "run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
            ));
            None
        }
        Err(err) => {
            checks.push(MedicCheck::error(
                "deployment state",
                err,
                "reinstall from the owning fleet template or re-register the deployment target with --allow-unverified",
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

fn is_missing_installed_deployment(error: &str) -> bool {
    error.starts_with("deployment target ") && error.contains(" is not installed on network ")
}

fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    match IcpCli::new(&options.icp, None, Some(options.network.clone())).compatible_version() {
        Ok(version) => MedicCheck::ok("icp cli", version, "-"),
        Err(err) => MedicCheck::error(
            "icp cli",
            err.to_string(),
            "install supported icp-cli or pass top-level --icp <path>",
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
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    let candid_path = role_candid_path(icp_root, &options.network, "root");
    let ready = query_canister_ready(
        &icp,
        &state.root_canister_id,
        &options.network,
        icp_root,
        candid_path.as_deref(),
    )
    .map_err(|err| err.to_string());

    match ready {
        Ok(true) => MedicCheck::ok(
            "root ready",
            "canic_ready=true",
            format!("run canic info list {}", options.deployment),
        ),
        Ok(false) => MedicCheck::warn(
            "root ready",
            "canic_ready=false",
            "wait briefly, then run canic info medic",
        ),
        Err(err) => MedicCheck::error("root ready", err, "run canic install"),
    }
}

fn render_medic_report(checks: &[MedicCheck]) -> String {
    let rows = checks
        .iter()
        .map(|check| {
            [
                check.name.clone(),
                medic_status_label(check.status).to_string(),
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

const fn medic_status_label(status: MedicStatus) -> &'static str {
    match status {
        MedicStatus::Ok => "ok",
        MedicStatus::Warn => "warn",
        MedicStatus::Error => "error",
    }
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

#[cfg(test)]
mod tests;
