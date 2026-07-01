//! Module: canic_cli::medic
//!
//! Responsibility: diagnose Canic project and installed-deployment readiness.
//! Does not own: deployment mutation, recovery, install-state persistence, or
//! canister control-plane changes.
//! Boundary: reads local project/deployment state and renders diagnostic-only
//! medic reports.

#[cfg(test)]
mod tests;

use crate::{
    auth::{self, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    blob_storage::{self, BlobStorageMedicStatus, BlobStorageMedicSummary},
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
        defaults::{default_icp, local_network},
        globals::{internal_icp_arg, internal_network_arg},
        help::print_help_or_version,
    },
    support::candid::role_candid_path,
    version_text,
};
use canic_core::protocol::{
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
};
use canic_host::{
    candid_endpoints::parse_candid_service_endpoints,
    canister_ready::query_canister_ready,
    icp::{IcpCli, local_canister_candid_path},
    icp_config::{inspect_canic_icp_yaml_from_root, resolve_current_canic_icp_root},
    install_root::{InstallState, discover_project_canic_config_choices},
    installed_deployment::read_installed_deployment_state_from_root,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{ffi::OsString, fs, path::Path};
use thiserror::Error as ThisError;

const MEDIC_REPORT_WIDTH: usize = 100;
const SCHEMA_VERSION: u8 = 1;
const PROJECT_COMMAND: &str = "project";
const DEPLOYMENT_COMMAND: &str = "deployment";
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const BLOB_STORAGE_ARG: &str = "blob-storage";
const AUTH_RENEWAL_ARG: &str = "auth-renewal";
const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";
const MEDIC_HELP_AFTER: &str = "\
Examples:
  canic medic
  canic medic project
  canic medic deployment test
  canic medic deployment test --blob-storage backend
  canic medic deployment test --auth-renewal rrkah-fqaaa-aaaaa-aaaaq-cai
  canic medic deployment test --json";

///
/// MedicCommandError
///

#[derive(Debug, ThisError)]
pub enum MedicCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to render medic JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("blocking preflight issues found")]
    ReportFailed,
}

impl MedicCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::ReportFailed => 1,
            Self::Json(_) => 3,
        }
    }

    pub const fn suppress_stderr(&self) -> bool {
        matches!(self, Self::ReportFailed)
    }
}

///
/// MedicOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct MedicOptions {
    scope: MedicScope,
    deployment: Option<String>,
    blob_storage: Option<String>,
    auth_renewal: Option<String>,
    json: bool,
    network: Option<String>,
    icp: String,
}

impl MedicOptions {
    fn parse<I>(args: I) -> Result<Self, MedicCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(medic_command(), args).map_err(|_| MedicCommandError::Usage(usage()))?;
        let json = matches.get_flag(JSON_ARG);
        let network = string_option(&matches, "network");
        let icp = string_option(&matches, "icp").unwrap_or_else(default_icp);

        match matches.subcommand() {
            None | Some((PROJECT_COMMAND, _)) => Ok(Self::project(json, network, icp)),
            Some((DEPLOYMENT_COMMAND, matches)) => Ok(Self {
                scope: MedicScope::Deployment,
                deployment: Some(required_string(matches, DEPLOYMENT_ARG)),
                blob_storage: string_option(matches, BLOB_STORAGE_ARG),
                auth_renewal: string_option(matches, AUTH_RENEWAL_ARG),
                json,
                network,
                icp,
            }),
            Some(_) => Err(MedicCommandError::Usage(usage())),
        }
    }

    const fn project(json: bool, network: Option<String>, icp: String) -> Self {
        Self {
            scope: MedicScope::Project,
            deployment: None,
            blob_storage: None,
            auth_renewal: None,
            json,
            network,
            icp,
        }
    }

    fn command_label(&self) -> String {
        match (&self.scope, &self.deployment) {
            (MedicScope::Project, _) => "canic medic project".to_string(),
            (MedicScope::Deployment, Some(deployment)) => {
                format!("canic medic deployment {deployment}")
            }
            (MedicScope::Deployment, None) => "canic medic deployment".to_string(),
        }
    }

    fn deployment_name(&self) -> &str {
        self.deployment
            .as_deref()
            .expect("deployment scope requires deployment name")
    }

    fn deployment_network(&self) -> String {
        self.network.clone().unwrap_or_else(local_network)
    }
}

pub fn run<I>(args: I) -> Result<(), MedicCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = MedicOptions::parse(args)?;
    let report = build_medic_report(&options);
    if options.json {
        println!("{}", render_medic_json(&report)?);
    } else {
        println!("{}", render_medic_text(&report));
    }
    if report.status == MedicStatus::Fail {
        return Err(MedicCommandError::ReportFailed);
    }
    Ok(())
}

fn medic_command() -> ClapCommand {
    ClapCommand::new("medic")
        .bin_name("canic medic")
        .disable_help_flag(true)
        .about("Diagnose Canic project and deployment preflight readiness")
        .arg(
            flag_arg(JSON_ARG)
                .long(JSON_ARG)
                .global(true)
                .help("Print JSON output"),
        )
        .arg(internal_network_arg().global(true))
        .arg(internal_icp_arg().global(true))
        .subcommand(project_command())
        .subcommand(deployment_command())
        .after_help(MEDIC_HELP_AFTER)
}

fn project_command() -> ClapCommand {
    ClapCommand::new(PROJECT_COMMAND)
        .disable_help_flag(true)
        .about("Run project-level medic checks")
}

fn deployment_command() -> ClapCommand {
    ClapCommand::new(DEPLOYMENT_COMMAND)
        .disable_help_flag(true)
        .about("Run deployment-level medic checks")
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(
            value_arg(BLOB_STORAGE_ARG)
                .long(BLOB_STORAGE_ARG)
                .value_name("canister-or-role")
                .help("Run targeted blob-storage billing readiness diagnostics"),
        )
        .arg(
            value_arg(AUTH_RENEWAL_ARG)
                .long(AUTH_RENEWAL_ARG)
                .value_name("issuer-principal")
                .help("Run targeted chain-key auth renewal drift diagnostics"),
        )
}

fn usage() -> String {
    render_usage(medic_command)
}

fn build_medic_report(options: &MedicOptions) -> MedicReport {
    let checks = match options.scope {
        MedicScope::Project => run_project_checks(options),
        MedicScope::Deployment => run_deployment_checks(options),
    };
    MedicReport::new(options, checks)
}

fn run_project_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = vec![
        check_icp_cli(options),
        check_icp_identity_session_cache_hint(),
    ];

    match resolve_current_canic_icp_root() {
        Ok(root) => {
            checks.push(MedicCheck::pass(
                MedicCategory::Environment,
                "project_root_resolved",
                "project_root",
                format!("resolved {}", root.display()),
                "none",
                MedicSource::Command,
            ));
            checks.extend(project_config_checks(&root, options));
        }
        Err(err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::Environment,
                "project_root_missing",
                "project_root",
                err.to_string(),
                "run from a Canic project root or set CANIC_ICP_ROOT",
                MedicSource::Command,
            ));
        }
    }

    checks.push(MedicCheck::not_evaluated(
        MedicCategory::DeploymentState,
        "deployment_not_selected",
        "deployment",
        "no deployment target was selected",
        "run canic medic deployment <deployment>",
        MedicSource::Command,
    ));
    checks
}

fn project_config_checks(root: &Path, options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = Vec::new();
    match discover_project_canic_config_choices(root) {
        Ok(configs) if configs.is_empty() => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "fleet_config_missing",
            "fleets",
            "no Canic fleet configs found",
            "create fleets/<fleet>/canic.toml or run canic fleet create <fleet>",
            MedicSource::FleetConfig,
        )),
        Ok(configs) => checks.push(MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "fleet_config_discovered",
            "fleets",
            format!("found {} Canic fleet config(s)", configs.len()),
            "none",
            MedicSource::FleetConfig,
        )),
        Err(err) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "fleet_config_missing",
            "fleets",
            err.to_string(),
            "repair Canic fleet config discovery",
            MedicSource::FleetConfig,
        )),
    }

    match inspect_canic_icp_yaml_from_root(root, None) {
        Ok(report) if report.icp_yaml_present => checks.push(MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "icp_yaml_present",
            "icp.yaml",
            format!("found {}", report.path.display()),
            "none",
            MedicSource::IcpConfig,
        )),
        Ok(report) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "icp_yaml_missing",
            "icp.yaml",
            format!("missing {}", report.path.display()),
            "create or repair icp.yaml from the project root",
            MedicSource::IcpConfig,
        )),
        Err(err) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "icp_yaml_missing",
            "icp.yaml",
            err.to_string(),
            "create or repair icp.yaml from the project root",
            MedicSource::IcpConfig,
        )),
    }

    if options.network.is_some() {
        checks.push(MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "local_network_explicit",
            "network",
            "network selected explicitly",
            "none",
            MedicSource::IcpConfig,
        ));
    } else {
        checks.push(MedicCheck::warn(
            MedicCategory::ProjectConfig,
            "local_network_implicit",
            "network",
            "no network was selected for project-level checks",
            "select an explicit network before deployment checks",
            MedicSource::IcpConfig,
        ));
    }

    checks
}

fn run_deployment_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = run_project_checks(options)
        .into_iter()
        .filter(|check| check.code != "deployment_not_selected")
        .collect::<Vec<_>>();
    let network = options.deployment_network();
    let icp_root = resolve_current_canic_icp_root().ok();

    checks.push(MedicCheck::pass(
        MedicCategory::Network,
        if options.network.is_some() {
            "local_network_explicit"
        } else {
            "local_network_implicit"
        },
        "network",
        network.clone(),
        "override with top-level --network <name>",
        MedicSource::Command,
    ));

    let state = match icp_root.as_deref().map_or_else(
        || Err("could not resolve ICP project root".to_string()),
        |root| {
            read_installed_deployment_state_from_root(&network, options.deployment_name(), root)
                .map_err(|err| err.to_string())
        },
    ) {
        Ok(state) => {
            checks.push(MedicCheck::pass(
                MedicCategory::DeploymentState,
                "deployment_target_found",
                "deployment",
                format!("{} installed", state.deployment_name),
                "run canic info list",
                MedicSource::InstalledDeployment,
            ));
            Some(state)
        }
        Err(err) if is_missing_installed_deployment(&err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                "no installed deployment found",
                "run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
                MedicSource::InstalledDeployment,
            ));
            None
        }
        Err(err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                err,
                "reinstall from the owning fleet template or re-register the deployment target with --allow-unverified",
                MedicSource::InstalledDeployment,
            ));
            None
        }
    };

    if let Some(state) = state.as_ref() {
        checks.extend(installed_deployment_state_checks(
            options,
            icp_root.as_deref(),
            state,
            &network,
        ));
    }

    if let Some(canister) = &options.blob_storage {
        checks.push(check_blob_storage_billing(options, canister, &network));
    } else {
        checks.push(check_blob_storage_not_selected(
            options,
            icp_root.as_deref(),
            &network,
        ));
    }

    if let Some(issuer) = &options.auth_renewal {
        checks.push(check_auth_renewal(options, issuer, &network));
    } else {
        checks.push(MedicCheck::not_evaluated(
            MedicCategory::Auth,
            "auth_renewal_not_selected",
            "auth_renewal",
            "no auth-renewal issuer was selected",
            "run canic medic deployment <deployment> --auth-renewal <issuer-principal>",
            MedicSource::Command,
        ));
    }

    checks
}

fn installed_deployment_state_checks(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    state: &InstallState,
    network: &str,
) -> Vec<MedicCheck> {
    let deployment_network = check_deployment_network(state, network);
    let deployment_network_matches = deployment_network.status != MedicStatus::Fail;
    let root_canister = check_root_canister_id(state);
    let root_canister_present = root_canister.status != MedicStatus::Fail;
    let root_readiness = if deployment_network_matches && root_canister_present {
        check_root_ready(options, icp_root, state, network)
    } else {
        check_root_readiness_not_evaluated(deployment_network_matches, root_canister_present)
    };

    vec![
        deployment_network,
        check_config_path(state),
        root_canister,
        root_readiness,
    ]
}

fn is_missing_installed_deployment(error: &str) -> bool {
    error.starts_with("deployment target ") && error.contains(" is not installed on network ")
}

fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    let network = options.network.clone();
    match IcpCli::new(&options.icp, None, network).compatible_version() {
        Ok(version) => MedicCheck::pass(
            MedicCategory::Environment,
            "icp_cli_ok",
            "icp",
            version,
            "none",
            MedicSource::IcpCli,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::Environment,
            "icp_cli_incompatible",
            "icp",
            err.to_string(),
            "install supported icp-cli or pass top-level --icp <path>",
            MedicSource::IcpCli,
        ),
    }
}

fn check_icp_identity_session_cache_hint() -> MedicCheck {
    MedicCheck::pass(
        MedicCategory::Environment,
        "icp_identity_session_hint",
        "icp_identity",
        ICP_SESSION_DETAIL,
        ICP_SESSION_NEXT,
        MedicSource::IcpCli,
    )
}

fn check_config_path(state: &InstallState) -> MedicCheck {
    if fs::metadata(&state.config_path).is_ok_and(|metadata| metadata.is_file()) {
        MedicCheck::pass(
            MedicCategory::DeploymentState,
            "recorded_config_path_found",
            "config",
            state.config_path.clone(),
            "none",
            MedicSource::InstalledDeployment,
        )
    } else {
        MedicCheck::fail(
            MedicCategory::DeploymentState,
            "recorded_config_path_missing",
            "config",
            format!("missing {}", state.config_path),
            "restore the config or reinstall the fleet",
            MedicSource::InstalledDeployment,
        )
    }
}

fn check_deployment_network(state: &InstallState, selected_network: &str) -> MedicCheck {
    if state.network == selected_network {
        MedicCheck::pass(
            MedicCategory::DeploymentState,
            "deployment_network_match",
            "network",
            format!("deployment record is scoped to {selected_network}"),
            "none",
            MedicSource::InstalledDeployment,
        )
    } else {
        MedicCheck::fail(
            MedicCategory::DeploymentState,
            "deployment_network_mismatch",
            "network",
            format!(
                "deployment record is scoped to {}, but medic selected {selected_network}",
                state.network
            ),
            "select the deployment record network or repair the installed deployment state",
            MedicSource::InstalledDeployment,
        )
    }
}

fn check_root_canister_id(state: &InstallState) -> MedicCheck {
    if state.root_canister_id.trim().is_empty() {
        MedicCheck::fail(
            MedicCategory::Topology,
            "root_canister_id_missing",
            "root",
            "installed deployment state does not record a root canister id",
            "re-register the deployment target or reinstall from the owning fleet template",
            MedicSource::InstalledDeployment,
        )
    } else {
        MedicCheck::pass(
            MedicCategory::Topology,
            "root_canister_id_present",
            "root",
            state.root_canister_id.clone(),
            "none",
            MedicSource::InstalledDeployment,
        )
    }
}

fn check_root_readiness_not_evaluated(
    deployment_network_matches: bool,
    root_canister_present: bool,
) -> MedicCheck {
    let detail = if !deployment_network_matches {
        "root readiness skipped because the deployment record network does not match the selected network"
    } else if !root_canister_present {
        "root readiness skipped because the deployment record has no root canister id"
    } else {
        "root readiness was not evaluated"
    };

    MedicCheck::not_evaluated(
        MedicCategory::Topology,
        "root_readiness_not_evaluated",
        "root",
        detail,
        "repair the blocking deployment-state check, then rerun canic medic deployment <deployment>",
        MedicSource::InstalledDeployment,
    )
}

fn check_root_ready(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    state: &InstallState,
    network: &str,
) -> MedicCheck {
    let mut icp = IcpCli::new(&options.icp, None, Some(network.to_string()));
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    let candid_path = role_candid_path(icp_root, network, "root");
    let ready = query_canister_ready(
        &icp,
        &state.root_canister_id,
        network,
        icp_root,
        candid_path.as_deref(),
    )
    .map_err(|err| err.to_string());

    match ready {
        Ok(true) => MedicCheck::pass(
            MedicCategory::Topology,
            "root_readiness_pass",
            "root",
            "canic_ready=true",
            "none",
            MedicSource::LocalReplica,
        ),
        Ok(false) => MedicCheck::warn(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            "canic_ready=false",
            "wait briefly, then run canic medic deployment <deployment>",
            MedicSource::LocalReplica,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            err,
            "run canic install",
            MedicSource::LocalReplica,
        ),
    }
}

fn check_blob_storage_billing(options: &MedicOptions, canister: &str, network: &str) -> MedicCheck {
    match blob_storage::medic_summary(options.deployment_name(), canister, network, &options.icp) {
        Ok(summary) => blob_storage_medic_check_from_summary(summary),
        Err(err) => MedicCheck::fail(
            MedicCategory::BlobStorage,
            "blob_storage_billing_unready",
            "blob_storage",
            err.to_string(),
            format!(
                "run canic blob-storage status {} {canister}",
                options.deployment_name()
            ),
            MedicSource::BlobStorageReadiness,
        ),
    }
}

fn check_blob_storage_not_selected(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    network: &str,
) -> MedicCheck {
    let next = icp_root
        .and_then(|root| {
            blob_storage_billing_roles_from_candid_dir(root, network)
                .into_iter()
                .next()
        })
        .map_or_else(
            || {
                "run canic medic deployment <deployment> --blob-storage <canister-or-role>"
                    .to_string()
            },
            |first| {
                format!(
                    "run canic medic deployment {} --blob-storage {first}",
                    options.deployment_name()
                )
            },
        );
    MedicCheck::not_evaluated(
        MedicCategory::BlobStorage,
        "blob_storage_not_selected",
        "blob_storage",
        "no blob-storage target was selected",
        next,
        MedicSource::Command,
    )
}

fn blob_storage_billing_roles_from_candid_dir(icp_root: &Path, network: &str) -> Vec<String> {
    let canisters_dir = icp_root.join(".icp").join(network).join("canisters");
    let Ok(entries) = fs::read_dir(canisters_dir) else {
        return Vec::new();
    };
    let mut roles = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|role| {
            let candid_path = local_canister_candid_path(icp_root, network, role);
            candid_path_declares_blob_storage_billing(&candid_path)
        })
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();
    roles
}

fn candid_path_declares_blob_storage_billing(path: &Path) -> bool {
    let Ok(candid) = fs::read_to_string(path) else {
        return false;
    };
    candid_declares_blob_storage_billing(&candid)
}

fn candid_declares_blob_storage_billing(candid: &str) -> bool {
    let Ok(endpoints) = parse_candid_service_endpoints(candid) else {
        return false;
    };
    [
        BLOB_STORAGE_STATUS,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
    ]
    .iter()
    .all(|method| endpoints.iter().any(|endpoint| endpoint.name == *method))
}

fn blob_storage_medic_check_from_summary(summary: BlobStorageMedicSummary) -> MedicCheck {
    match summary.status {
        BlobStorageMedicStatus::Ready => MedicCheck::pass(
            MedicCategory::BlobStorage,
            "blob_storage_billing_ready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
        BlobStorageMedicStatus::Warning => MedicCheck::warn(
            MedicCategory::BlobStorage,
            "blob_storage_billing_unready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
        BlobStorageMedicStatus::Blocked => MedicCheck::fail(
            MedicCategory::BlobStorage,
            "blob_storage_billing_unready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
    }
}

fn check_auth_renewal(options: &MedicOptions, issuer: &str, network: &str) -> MedicCheck {
    match auth::renewal_medic_summary(options.deployment_name(), issuer, network, &options.icp) {
        Ok(summary) => auth_renewal_medic_check_from_summary(summary),
        Err(err) => MedicCheck::fail(
            MedicCategory::Auth,
            "auth_renewal_drift_fail",
            "auth_renewal",
            err.to_string(),
            format!(
                "run canic auth renewal status {} --issuer {issuer}",
                options.deployment_name()
            ),
            MedicSource::AuthRenewal,
        ),
    }
}

fn auth_renewal_medic_check_from_summary(summary: AuthRenewalMedicSummary) -> MedicCheck {
    match summary.status {
        AuthRenewalMedicStatus::Ready => MedicCheck::pass(
            MedicCategory::Auth,
            "auth_renewal_ready",
            "auth_renewal",
            summary.detail,
            summary.next,
            MedicSource::AuthRenewal,
        ),
        AuthRenewalMedicStatus::Warning => MedicCheck::warn(
            MedicCategory::Auth,
            "auth_renewal_drift_warn",
            "auth_renewal",
            summary.detail,
            summary.next,
            MedicSource::AuthRenewal,
        ),
    }
}

fn render_medic_json(report: &MedicReport) -> Result<String, MedicCommandError> {
    serde_json::to_string_pretty(report).map_err(MedicCommandError::from)
}

fn render_medic_text(report: &MedicReport) -> String {
    let mut lines = vec![
        report.command.clone(),
        format!("status: {}", report.status.label()),
        format!(
            "network: {}",
            report.network.as_deref().unwrap_or("not selected")
        ),
        format!(
            "deployment: {}",
            report.deployment.as_deref().unwrap_or("not selected")
        ),
    ];

    for check in ordered_checks(&report.checks) {
        lines.push(String::new());
        lines.push(format!(
            "{} [{}] {}",
            check.category.label(),
            check.status.label(),
            check.code
        ));
        push_medic_field(&mut lines, "subject", &check.subject);
        push_medic_field(&mut lines, "detail", &check.detail);
        push_medic_field(&mut lines, "next", &check.next);
        push_medic_field(&mut lines, "source", check.source.label());
    }
    lines.join("\n")
}

fn ordered_checks(checks: &[MedicCheck]) -> Vec<&MedicCheck> {
    let mut checks = checks.iter().collect::<Vec<_>>();
    checks.sort_by_key(|check| check.category.order());
    checks
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

///
/// MedicReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MedicReport {
    schema_version: u8,
    command: String,
    scope: MedicScope,
    network: Option<String>,
    deployment: Option<String>,
    status: MedicStatus,
    checks: Vec<MedicCheck>,
}

impl MedicReport {
    fn new(options: &MedicOptions, checks: Vec<MedicCheck>) -> Self {
        let status = aggregate_status(&checks);
        let network = match options.scope {
            MedicScope::Project => options.network.clone(),
            MedicScope::Deployment => Some(options.deployment_network()),
        };
        Self {
            schema_version: SCHEMA_VERSION,
            command: options.command_label(),
            scope: options.scope,
            network,
            deployment: options.deployment.clone(),
            status,
            checks: ordered_checks(&checks).into_iter().cloned().collect(),
        }
    }
}

fn aggregate_status(checks: &[MedicCheck]) -> MedicStatus {
    if checks.is_empty()
        || checks
            .iter()
            .all(|check| check.status == MedicStatus::NotEvaluated)
    {
        return MedicStatus::NotEvaluated;
    }
    if checks.iter().any(|check| check.status == MedicStatus::Fail) {
        return MedicStatus::Fail;
    }
    if checks.iter().any(|check| check.status == MedicStatus::Warn) {
        return MedicStatus::Warn;
    }
    MedicStatus::Pass
}

///
/// MedicCheck
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct MedicCheck {
    category: MedicCategory,
    code: String,
    status: MedicStatus,
    subject: String,
    detail: String,
    next: String,
    source: MedicSource,
}

impl MedicCheck {
    fn pass(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Pass,
            subject,
            detail,
            next,
            source,
        )
    }

    fn warn(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Warn,
            subject,
            detail,
            next,
            source,
        )
    }

    fn fail(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Fail,
            subject,
            detail,
            next,
            source,
        )
    }

    fn not_evaluated(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::NotEvaluated,
            subject,
            detail,
            next,
            source,
        )
    }

    fn new(
        category: MedicCategory,
        code: impl Into<String>,
        status: MedicStatus,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self {
            category,
            code: code.into(),
            status,
            subject: subject.into(),
            detail: detail.into(),
            next: next.into(),
            source,
        }
    }
}

///
/// MedicScope
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MedicScope {
    Project,
    Deployment,
}

///
/// MedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MedicStatus {
    Pass,
    Warn,
    Fail,
    NotEvaluated,
}

impl MedicStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

///
/// MedicCategory
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MedicCategory {
    Environment,
    ProjectConfig,
    Network,
    DeploymentState,
    Topology,
    #[expect(dead_code, reason = "0.78 report schema reserves artifact checks")]
    Artifact,
    #[expect(dead_code, reason = "0.78 report schema reserves feature checks")]
    Feature,
    Auth,
    BlobStorage,
    #[expect(dead_code, reason = "0.78 report schema reserves runtime checks")]
    Runtime,
}

impl MedicCategory {
    const fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::ProjectConfig => "project_config",
            Self::Network => "network",
            Self::DeploymentState => "deployment_state",
            Self::Topology => "topology",
            Self::Artifact => "artifact",
            Self::Feature => "feature",
            Self::Auth => "auth",
            Self::BlobStorage => "blob_storage",
            Self::Runtime => "runtime",
        }
    }

    const fn order(self) -> usize {
        match self {
            Self::Environment => 0,
            Self::ProjectConfig => 1,
            Self::Network => 2,
            Self::DeploymentState => 3,
            Self::Topology => 4,
            Self::Artifact => 5,
            Self::Feature => 6,
            Self::Auth => 7,
            Self::BlobStorage => 8,
            Self::Runtime => 9,
        }
    }
}

///
/// MedicSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MedicSource {
    Command,
    IcpCli,
    IcpConfig,
    FleetConfig,
    InstalledDeployment,
    #[expect(
        dead_code,
        reason = "0.78 report schema reserves deployment-truth checks"
    )]
    DeploymentTruth,
    LocalReplica,
    BlobStorageReadiness,
    AuthRenewal,
}

impl MedicSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::IcpCli => "icp_cli",
            Self::IcpConfig => "icp_config",
            Self::FleetConfig => "fleet_config",
            Self::InstalledDeployment => "installed_deployment",
            Self::DeploymentTruth => "deployment_truth",
            Self::LocalReplica => "local_replica",
            Self::BlobStorageReadiness => "blob_storage_readiness",
            Self::AuthRenewal => "auth_renewal",
        }
    }
}
