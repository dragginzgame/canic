//! Module: canic_cli::medic
//!
//! Responsibility: diagnose local installed-deployment setup for operators.
//! Does not own: deployment mutation, recovery, install state persistence, or
//! canister control-plane changes.
//! Boundary: reads local deployment state, checks local CLI readiness, and
//! queries root readiness for display.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{parse_matches, render_usage, required_string, string_option_or_else, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    support::candid::role_candid_path,
    version_text,
};
use canic_host::{
    canister_ready::query_canister_ready, icp::IcpCli, icp_config::resolve_current_canic_icp_root,
    install_root::InstallState, installed_deployment::read_installed_deployment_state_from_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::Path};
use thiserror::Error as ThisError;

const MEDIC_REPORT_WIDTH: usize = 100;
const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";
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
        let matches = parse_matches(info_medic_command(), args)
            .map_err(|_| MedicCommandError::Usage(info_usage()))?;

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
    ClapCommand::new("medic")
        .bin_name("canic info medic")
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
        .after_help(INFO_MEDIC_HELP_AFTER)
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
    checks.push(check_icp_identity_session_cache_hint());

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

fn check_icp_identity_session_cache_hint() -> MedicCheck {
    MedicCheck::ok("icp identity session", ICP_SESSION_DETAIL, ICP_SESSION_NEXT)
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
    let mut lines = Vec::new();
    for (index, check) in checks.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.push(format!(
            "{} [{}]",
            check.name,
            medic_status_label(check.status)
        ));
        push_medic_field(&mut lines, "detail", &check.detail);
        if check.next != "-" {
            push_medic_field(&mut lines, "next", &check.next);
        }
    }
    lines.join("\n")
}

fn push_medic_field(lines: &mut Vec<String>, label: &str, value: &str) {
    let prefix = format!("  {label}: ");
    let continuation_prefix = " ".repeat(prefix.chars().count());
    let width = MEDIC_REPORT_WIDTH.saturating_sub(prefix.chars().count());

    for (index, line) in wrap_medic_text(value, width).into_iter().enumerate() {
        if index == 0 {
            lines.push(format!("{prefix}{line}"));
        } else if line.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(format!("{continuation_prefix}{line}"));
        }
    }
}

fn wrap_medic_text(value: &str, width: usize) -> Vec<String> {
    let wrapped = value
        .lines()
        .flat_map(|line| wrap_medic_line(line, width))
        .collect::<Vec<_>>();
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn wrap_medic_line(line: &str, width: usize) -> Vec<String> {
    if line.trim().is_empty() {
        return vec![String::new()];
    }

    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        let candidate_width =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if candidate_width <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
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
