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
    auth::{self, AuthCommandError, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    blob_storage::{
        self, BlobStorageCommandError, BlobStorageMedicStatus, BlobStorageMedicSummary,
    },
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
        defaults::{default_icp, local_network},
        globals::{
            INTERNAL_ICP_OPTION, INTERNAL_NETWORK_OPTION, internal_icp_arg, internal_network_arg,
        },
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
    deployment_truth::{
        DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    },
    icp::{IcpCli, IcpCommandError, local_canister_candid_path},
    icp_config::{inspect_canic_icp_yaml_from_root, resolve_current_canic_icp_root},
    install_root::{
        InstallState, discover_project_canic_config_choices,
        latest_deployment_truth_receipt_path_from_root,
    },
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        InstalledDeploymentSource, read_installed_deployment_state_from_root,
        resolve_installed_deployment_from_root,
    },
    release_set::{ConfiguredRoleLifecycle, configured_fleet_name, configured_role_lifecycle},
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;
use toml::Value as TomlValue;

const MEDIC_REPORT_WIDTH: usize = 100;
const SCHEMA_VERSION: u8 = 1;
const PROJECT_COMMAND: &str = "project";
const DEPLOYMENT_COMMAND: &str = "deployment";
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const BLOB_STORAGE_ARG: &str = "blob-storage";
const AUTH_RENEWAL_ARG: &str = "auth-renewal";
const PACKAGE_MANIFEST_FILE: &str = "Cargo.toml";
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
    if medic_subcommand_help_requested(&args) {
        println!("{}", usage());
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

fn medic_subcommand_help_requested(args: &[OsString]) -> bool {
    let mut index = skip_medic_options(args, 0);
    let Some(PROJECT_COMMAND | DEPLOYMENT_COMMAND) = args.get(index).and_then(|arg| arg.to_str())
    else {
        return false;
    };
    index = skip_medic_options(args, index + 1);
    args.get(index).is_some_and(is_medic_help_arg)
}

fn skip_medic_options(args: &[OsString], mut index: usize) -> usize {
    while let Some(arg) = args.get(index).and_then(|arg| arg.to_str()) {
        match arg {
            "--json" => index += 1,
            INTERNAL_ICP_OPTION | INTERNAL_NETWORK_OPTION => index += 2,
            _ => break,
        }
    }
    index
}

fn is_medic_help_arg(arg: &OsString) -> bool {
    matches!(arg.to_str(), Some("help" | "--help" | "-h"))
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
    match options.scope {
        MedicScope::Project => MedicReport::new(options, run_project_checks(options)),
        MedicScope::Deployment => {
            let context = deployment_medic_context(options);
            let network = Some(context.network.clone());
            MedicReport::with_network(options, network, run_deployment_checks(options, &context))
        }
    }
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
        Ok(configs) => {
            checks.push(MedicCheck::pass(
                MedicCategory::ProjectConfig,
                "fleet_config_discovered",
                "fleets",
                format!("found {} Canic fleet config(s)", configs.len()),
                "none",
                MedicSource::FleetConfig,
            ));
            checks.extend(project_config_quality_checks(root, &configs));
        }
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

    if let Some(network) = project_network_selection_check(options) {
        checks.push(network);
    }

    checks
}

fn project_network_selection_check(options: &MedicOptions) -> Option<MedicCheck> {
    if options.scope != MedicScope::Project {
        return None;
    }

    Some(if options.network.is_some() {
        MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "local_network_explicit",
            "network",
            "network selected explicitly",
            "none",
            MedicSource::IcpConfig,
        )
    } else {
        MedicCheck::warn(
            MedicCategory::ProjectConfig,
            "local_network_implicit",
            "network",
            "no network was selected for project-level checks",
            "select an explicit network before deployment checks",
            MedicSource::IcpConfig,
        )
    })
}

fn project_config_quality_checks(root: &Path, configs: &[PathBuf]) -> Vec<MedicCheck> {
    configs
        .iter()
        .flat_map(|config| fleet_config_quality_checks(root, config))
        .collect()
}

fn fleet_config_quality_checks(root: &Path, config: &Path) -> Vec<MedicCheck> {
    let config_display = display_medic_path(root, config);
    let fleet = match configured_fleet_name(config) {
        Ok(fleet) => fleet,
        Err(err) => {
            return vec![MedicCheck::fail(
                MedicCategory::ProjectConfig,
                "fleet_config_missing",
                config_display,
                err.to_string(),
                "repair the fleet config before running deployment checks",
                MedicSource::FleetConfig,
            )];
        }
    };
    let roles = match configured_role_lifecycle(config) {
        Ok(roles) => roles,
        Err(err) => {
            return vec![MedicCheck::fail(
                MedicCategory::ProjectConfig,
                "fleet_config_missing",
                config_display,
                err.to_string(),
                "repair the fleet config before running deployment checks",
                MedicSource::FleetConfig,
            )];
        }
    };

    roles
        .iter()
        .flat_map(|role| {
            let mut checks = vec![check_role_package_metadata(root, config, role, &fleet)];
            if !role.attached {
                checks.push(check_declared_role_not_deployable(root, config, role));
            }
            checks
        })
        .collect()
}

fn check_role_package_metadata(
    root: &Path,
    config: &Path,
    role: &ConfiguredRoleLifecycle,
    fleet: &str,
) -> MedicCheck {
    let manifest = role_package_manifest_path(config, &role.package);
    match canic_package_metadata(&manifest) {
        Ok(metadata) if metadata.fleet == fleet && metadata.role == role.role => MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "role_package_metadata_present",
            role.display.clone(),
            format!(
                "{} declares [package.metadata.canic] fleet={} role={}",
                display_medic_path(root, &manifest),
                metadata.fleet,
                metadata.role
            ),
            "none",
            MedicSource::FleetConfig,
        ),
        Ok(metadata) => MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "role_package_metadata_missing",
            role.display.clone(),
            format!(
                "{} declares [package.metadata.canic] fleet={} role={}, expected fleet={} role={}",
                display_medic_path(root, &manifest),
                metadata.fleet,
                metadata.role,
                fleet,
                role.role
            ),
            "update package metadata or repair the fleet role declaration",
            MedicSource::FleetConfig,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "role_package_metadata_missing",
            role.display.clone(),
            err,
            "add matching [package.metadata.canic] fleet and role metadata",
            MedicSource::FleetConfig,
        ),
    }
}

fn check_declared_role_not_deployable(
    root: &Path,
    config: &Path,
    role: &ConfiguredRoleLifecycle,
) -> MedicCheck {
    MedicCheck::warn(
        MedicCategory::ProjectConfig,
        "declared_role_not_deployable",
        role.display.clone(),
        format!(
            "role is declared in {} but is not attached to topology",
            display_medic_path(root, config)
        ),
        format!(
            "run canic fleet role attach {} {} --subnet <subnet>, or remove the declaration",
            role.fleet, role.role
        ),
        MedicSource::FleetConfig,
    )
}

fn role_package_manifest_path(config: &Path, package: &str) -> PathBuf {
    let package_path = PathBuf::from(package);
    let path = if package_path.is_absolute() {
        package_path
    } else {
        config
            .parent()
            .map_or_else(|| PathBuf::from(package), |parent| parent.join(package))
    };
    if path.file_name().and_then(|name| name.to_str()) == Some(PACKAGE_MANIFEST_FILE) {
        path
    } else {
        path.join(PACKAGE_MANIFEST_FILE)
    }
}

fn canic_package_metadata(path: &Path) -> Result<CanicPackageMetadata, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let manifest = toml::from_str::<TomlValue>(&source)
        .map_err(|err| format!("invalid {}: {err}", path.display()))?;
    let fleet = manifest_string(&manifest, &["package", "metadata", "canic", "fleet"], path)?;
    let role = manifest_string(&manifest, &["package", "metadata", "canic", "role"], path)?;
    Ok(CanicPackageMetadata { fleet, role })
}

fn manifest_string(
    manifest: &TomlValue,
    path: &[&str],
    manifest_path: &Path,
) -> Result<String, String> {
    let mut value = manifest;
    for segment in path {
        value = value
            .get(*segment)
            .ok_or_else(|| format!("missing {} in {}", path.join("."), manifest_path.display()))?;
    }
    value.as_str().map(ToString::to_string).ok_or_else(|| {
        format!(
            "{} must be a string in {}",
            path.join("."),
            manifest_path.display()
        )
    })
}

fn display_medic_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

struct CanicPackageMetadata {
    fleet: String,
    role: String,
}

///
/// DeploymentMedicContext
///

struct DeploymentMedicContext {
    icp_root: Option<PathBuf>,
    network: String,
    network_check: MedicCheck,
}

fn deployment_medic_context(options: &MedicOptions) -> DeploymentMedicContext {
    let icp_root = resolve_current_canic_icp_root().ok();
    let (network, network_check) = deployment_network_selection(options, icp_root.as_deref());
    DeploymentMedicContext {
        icp_root,
        network,
        network_check,
    }
}

fn deployment_network_selection(
    options: &MedicOptions,
    icp_root: Option<&Path>,
) -> (String, MedicCheck) {
    if let Some(network) = &options.network {
        return (
            network.clone(),
            MedicCheck::pass(
                MedicCategory::Network,
                "local_network_explicit",
                "network",
                network.clone(),
                "none",
                MedicSource::Command,
            ),
        );
    }

    if let Some(network) =
        icp_root.and_then(|root| recorded_deployment_network(root, options.deployment_name()))
    {
        return (
            network.clone(),
            MedicCheck::pass(
                MedicCategory::Network,
                "deployment_network_from_record",
                "network",
                network,
                "override with top-level --network <name>",
                MedicSource::InstalledDeployment,
            ),
        );
    }

    let network = local_network();
    (
        network.clone(),
        MedicCheck::pass(
            MedicCategory::Network,
            "local_network_implicit",
            "network",
            network,
            "override with top-level --network <name>",
            MedicSource::Command,
        ),
    )
}

fn recorded_deployment_network(icp_root: &Path, deployment: &str) -> Option<String> {
    let canic_dir = icp_root.join(".canic");
    let mut networks = fs::read_dir(canic_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|network| {
            icp_root
                .join(".canic")
                .join(network)
                .join("deployments")
                .join(format!("{deployment}.json"))
                .is_file()
        })
        .collect::<Vec<_>>();
    networks.sort();
    networks.dedup();
    match networks.as_slice() {
        [network] => Some(network.clone()),
        _ => None,
    }
}

fn run_deployment_checks(
    options: &MedicOptions,
    context: &DeploymentMedicContext,
) -> Vec<MedicCheck> {
    let mut checks = run_project_checks(options)
        .into_iter()
        .filter(|check| check.code != "deployment_not_selected")
        .collect::<Vec<_>>();
    let network = &context.network;
    let icp_root = context.icp_root.as_deref();

    checks.push(context.network_check.clone());

    let state = match icp_root.map_or_else(
        || Err("could not resolve ICP project root".to_string()),
        |root| {
            read_installed_deployment_state_from_root(network, options.deployment_name(), root)
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
                deploy_plan_then(
                    options.deployment_name(),
                    "then run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
                ),
                MedicSource::InstalledDeployment,
            ));
            if let Some(root) = icp_root {
                checks.extend(deployment_name_conflation_checks(
                    root,
                    options.deployment_name(),
                ));
            }
            None
        }
        Err(err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                err,
                deploy_plan_then(
                    options.deployment_name(),
                    "then reinstall from the owning fleet template or re-register the deployment target with --allow-unverified",
                ),
                MedicSource::InstalledDeployment,
            ));
            None
        }
    };

    if let Some(state) = state.as_ref() {
        checks.extend(installed_deployment_state_checks(
            options, icp_root, state, network,
        ));
    }

    if let Some(canister) = &options.blob_storage {
        checks.push(check_blob_storage_billing(options, canister, network));
    } else {
        checks.push(check_blob_storage_not_selected(options, icp_root, network));
    }

    if let Some(issuer) = &options.auth_renewal {
        checks.push(check_auth_renewal(options, issuer, network));
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

fn deployment_name_conflation_checks(root: &Path, deployment: &str) -> Vec<MedicCheck> {
    let Ok(configs) = discover_project_canic_config_choices(root) else {
        return Vec::new();
    };

    let mut checks = Vec::new();
    for config in configs {
        if let Ok(fleet) = configured_fleet_name(&config)
            && fleet == deployment
        {
            checks.push(MedicCheck::warn(
                MedicCategory::ProjectConfig,
                "fleet_name_deployment_name_conflated",
                deployment,
                format!(
                    "selected deployment target matches fleet template {} in {}",
                    fleet,
                    display_medic_path(root, &config)
                ),
                deploy_plan_then(
                    deployment,
                    format!(
                        "then run canic install {fleet}, or choose an installed deployment target"
                    ),
                ),
                MedicSource::FleetConfig,
            ));
        }

        if let Ok(roles) = configured_role_lifecycle(&config) {
            checks.extend(roles.into_iter().filter_map(|role| {
                (role.role == deployment).then(|| {
                    MedicCheck::warn(
                        MedicCategory::ProjectConfig,
                        "role_name_deployment_name_conflated",
                        deployment,
                        format!(
                            "selected deployment target matches role {} in {}",
                            role.display,
                            display_medic_path(root, &config)
                        ),
                        "pass an installed deployment target, not a role name",
                        MedicSource::FleetConfig,
                    )
                })
            }));
        }
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
        check_deployment_truth_receipt(icp_root, state, network),
        root_canister,
        check_deployment_registry_observation(
            options,
            icp_root,
            state,
            network,
            deployment_network_matches,
            root_canister_present,
        ),
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
        Err(err) => icp_cli_error_check(err),
    }
}

fn icp_cli_error_check(error: IcpCommandError) -> MedicCheck {
    let code = match error {
        IcpCommandError::MissingCli { .. } => "icp_cli_missing",
        IcpCommandError::IncompatibleCliVersion { .. }
        | IcpCommandError::Io(_)
        | IcpCommandError::Failed { .. }
        | IcpCommandError::Json { .. }
        | IcpCommandError::SnapshotIdUnavailable { .. } => "icp_cli_incompatible",
    };

    MedicCheck::fail(
        MedicCategory::Environment,
        code,
        "icp",
        error.to_string(),
        "install supported icp-cli or pass top-level --icp <path>",
        MedicSource::IcpCli,
    )
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

fn check_deployment_truth_receipt(
    icp_root: Option<&Path>,
    state: &InstallState,
    network: &str,
) -> MedicCheck {
    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::DeploymentState,
            "deployment_truth_incomplete",
            "deployment_truth",
            "deployment truth receipt lookup skipped because the project root was not resolved",
            "run from a Canic project root or set CANIC_ICP_ROOT",
            MedicSource::DeploymentTruth,
        );
    };

    let receipt_path = match latest_deployment_truth_receipt_path_from_root(
        root,
        network,
        &state.deployment_name,
    ) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return MedicCheck::warn(
                MedicCategory::DeploymentState,
                "deployment_truth_incomplete",
                "deployment_truth",
                format!(
                    "no deployment-truth receipt found for {} on {network}",
                    state.deployment_name
                ),
                format!(
                    "{}; then run canic deploy check {} before mutating the deployment",
                    deploy_plan_next(&state.deployment_name),
                    state.deployment_name
                ),
                MedicSource::DeploymentTruth,
            );
        }
        Err(err) => {
            return MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_truth_incomplete",
                "deployment_truth",
                err.to_string(),
                "repair deployment-truth receipt state, then rerun canic medic deployment <deployment>",
                MedicSource::DeploymentTruth,
            );
        }
    };

    let receipt = match fs::read(&receipt_path)
        .map_err(|err| format!("failed to read {}: {err}", receipt_path.display()))
        .and_then(|bytes| {
            serde_json::from_slice::<DeploymentReceiptV1>(&bytes)
                .map_err(|err| format!("invalid {}: {err}", receipt_path.display()))
        }) {
        Ok(receipt) => receipt,
        Err(err) => {
            return MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_truth_incomplete",
                "deployment_truth",
                err,
                "repair or remove the invalid deployment-truth receipt",
                MedicSource::DeploymentTruth,
            );
        }
    };

    deployment_truth_receipt_check(root, &receipt_path, &receipt, &state.deployment_name)
}

fn deployment_truth_receipt_check(
    root: &Path,
    receipt_path: &Path,
    receipt: &DeploymentReceiptV1,
    deployment: &str,
) -> MedicCheck {
    let detail = format!(
        "{}; status={}; result={}; final_inventory={}",
        display_medic_path(root, receipt_path),
        deployment_execution_status_label(receipt.operation_status),
        deployment_command_result_label(&receipt.command_result),
        receipt.final_inventory_id.as_deref().unwrap_or("<missing>")
    );

    if receipt.operation_status == DeploymentExecutionStatusV1::Complete
        && receipt.command_result == DeploymentCommandResultV1::Succeeded
        && receipt.final_inventory_id.is_some()
    {
        return MedicCheck::pass(
            MedicCategory::DeploymentState,
            "deployment_truth_complete",
            "deployment_truth",
            detail,
            "none",
            MedicSource::DeploymentTruth,
        );
    }

    let next = format!("run canic deploy inspect resume-report {deployment}");
    match receipt.operation_status {
        DeploymentExecutionStatusV1::PartiallyApplied
        | DeploymentExecutionStatusV1::FailedAfterMutation => MedicCheck::fail(
            MedicCategory::DeploymentState,
            "deployment_truth_incomplete",
            "deployment_truth",
            detail,
            next,
            MedicSource::DeploymentTruth,
        ),
        DeploymentExecutionStatusV1::Complete => MedicCheck::fail(
            MedicCategory::DeploymentState,
            "deployment_truth_incomplete",
            "deployment_truth",
            detail,
            "repair the inconsistent deployment-truth receipt before mutating the deployment",
            MedicSource::DeploymentTruth,
        ),
        DeploymentExecutionStatusV1::NotStarted
        | DeploymentExecutionStatusV1::InProgress
        | DeploymentExecutionStatusV1::FailedBeforeMutation => MedicCheck::warn(
            MedicCategory::DeploymentState,
            "deployment_truth_incomplete",
            "deployment_truth",
            detail,
            next,
            MedicSource::DeploymentTruth,
        ),
    }
}

const fn deployment_execution_status_label(status: DeploymentExecutionStatusV1) -> &'static str {
    match status {
        DeploymentExecutionStatusV1::NotStarted => "not_started",
        DeploymentExecutionStatusV1::InProgress => "in_progress",
        DeploymentExecutionStatusV1::FailedBeforeMutation => "failed_before_mutation",
        DeploymentExecutionStatusV1::PartiallyApplied => "partially_applied",
        DeploymentExecutionStatusV1::FailedAfterMutation => "failed_after_mutation",
        DeploymentExecutionStatusV1::Complete => "complete",
    }
}

fn deployment_command_result_label(result: &DeploymentCommandResultV1) -> String {
    match result {
        DeploymentCommandResultV1::NotFinished => "not_finished".to_string(),
        DeploymentCommandResultV1::Succeeded => "succeeded".to_string(),
        DeploymentCommandResultV1::Failed { code, .. } => format!("failed:{code}"),
    }
}

fn check_deployment_registry_observation(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    state: &InstallState,
    network: &str,
    deployment_network_matches: bool,
    root_canister_present: bool,
) -> MedicCheck {
    if !deployment_network_matches || !root_canister_present {
        return check_deployment_registry_not_evaluated(
            deployment_network_matches,
            root_canister_present,
        );
    }

    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::Topology,
            "deployment_registry_not_evaluated",
            "registry",
            "deployment registry observation skipped because the project root was not resolved",
            "run from a Canic project root or set CANIC_ICP_ROOT",
            MedicSource::InstalledDeployment,
        );
    };

    let request = InstalledDeploymentRequest {
        deployment: state.deployment_name.clone(),
        network: network.to_string(),
        icp: options.icp.clone(),
        detect_lost_local_root: true,
    };

    match resolve_installed_deployment_from_root(&request, root) {
        Ok(resolution) => deployment_registry_observed_check(&resolution),
        Err(err) => deployment_registry_error_check(err),
    }
}

fn check_deployment_registry_not_evaluated(
    deployment_network_matches: bool,
    root_canister_present: bool,
) -> MedicCheck {
    let detail = if !deployment_network_matches {
        "deployment registry observation skipped because the deployment record network does not match the selected network"
    } else if !root_canister_present {
        "deployment registry observation skipped because the deployment record has no root canister id"
    } else {
        "deployment registry observation was not evaluated"
    };

    MedicCheck::not_evaluated(
        MedicCategory::Topology,
        "deployment_registry_not_evaluated",
        "registry",
        detail,
        "repair the blocking deployment-state check, then rerun canic medic deployment <deployment>",
        MedicSource::InstalledDeployment,
    )
}

fn deployment_registry_observed_check(resolution: &InstalledDeploymentResolution) -> MedicCheck {
    let entries = resolution.registry.entries.len();
    let roles = resolution.topology.roles_by_canister.len();
    let detail = format!(
        "root={}; entries={entries}; roles={roles}",
        resolution.registry.root_canister_id
    );
    let source = installed_deployment_source_for_medic(resolution.source);

    if entries == 0 {
        return MedicCheck::warn(
            MedicCategory::Topology,
            "deployment_registry_empty",
            "registry",
            detail,
            format!(
                "{}; then run canic deploy check {}",
                deploy_plan_next(&resolution.state.deployment_name),
                resolution.state.deployment_name
            ),
            source,
        );
    }

    MedicCheck::pass(
        MedicCategory::Topology,
        "deployment_registry_observed",
        "registry",
        detail,
        "none",
        source,
    )
}

fn deploy_plan_next(deployment: &str) -> String {
    format!("run canic deploy plan {deployment} to inspect desired deployment shape")
}

fn deploy_plan_then(deployment: &str, next: impl AsRef<str>) -> String {
    format!("{}; {}", deploy_plan_next(deployment), next.as_ref())
}

const fn installed_deployment_source_for_medic(source: InstalledDeploymentSource) -> MedicSource {
    match source {
        InstalledDeploymentSource::LocalReplica => MedicSource::LocalReplica,
        InstalledDeploymentSource::IcpCli => MedicSource::IcpCli,
    }
}

fn deployment_registry_error_check(error: InstalledDeploymentError) -> MedicCheck {
    let source = match error {
        InstalledDeploymentError::ReplicaQuery(_)
        | InstalledDeploymentError::LostLocalDeployment { .. } => MedicSource::LocalReplica,
        InstalledDeploymentError::IcpFailed { .. } => MedicSource::IcpCli,
        InstalledDeploymentError::NoInstalledDeployment { .. }
        | InstalledDeploymentError::InstallState(_)
        | InstalledDeploymentError::Registry(_)
        | InstalledDeploymentError::Io(_) => MedicSource::InstalledDeployment,
    };

    MedicCheck::fail(
        MedicCategory::Topology,
        "deployment_registry_unavailable",
        "registry",
        error.to_string(),
        "run canic status, then rerun canic medic deployment <deployment>",
        source,
    )
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
    let source = root_readiness_source(network);
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
            source,
        ),
        Ok(false) => MedicCheck::warn(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            "canic_ready=false",
            "wait briefly, then run canic medic deployment <deployment>",
            source,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            err,
            "run canic install",
            source,
        ),
    }
}

fn root_readiness_source(network: &str) -> MedicSource {
    if network == local_network() {
        MedicSource::LocalReplica
    } else {
        MedicSource::IcpCli
    }
}

fn check_blob_storage_billing(options: &MedicOptions, canister: &str, network: &str) -> MedicCheck {
    match blob_storage::medic_summary(options.deployment_name(), canister, network, &options.icp) {
        Ok(summary) => blob_storage_medic_check_from_summary(summary),
        Err(err) => blob_storage_medic_error_check(err, options.deployment_name(), canister),
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

fn blob_storage_medic_error_check(
    error: BlobStorageCommandError,
    deployment: &str,
    canister: &str,
) -> MedicCheck {
    let (code, next) = match &error {
        BlobStorageCommandError::UnknownTarget { .. } => (
            "blob_storage_target_missing",
            format!(
                "choose a registered blob-storage role or canister for deployment {deployment}"
            ),
        ),
        BlobStorageCommandError::AmbiguousRole { .. } => (
            "blob_storage_target_ambiguous",
            "use one canister principal instead of an ambiguous role".to_string(),
        ),
        BlobStorageCommandError::CandidUnavailable { .. }
        | BlobStorageCommandError::MethodUnavailable { .. } => (
            "blob_storage_target_not_blob_storage",
            "select a canister that exposes blob-storage billing readiness endpoints".to_string(),
        ),
        _ => (
            "blob_storage_billing_unready",
            format!("run canic blob-storage status {deployment} {canister}"),
        ),
    };

    MedicCheck::fail(
        MedicCategory::BlobStorage,
        code,
        "blob_storage",
        error.to_string(),
        next,
        MedicSource::BlobStorageReadiness,
    )
}

fn check_auth_renewal(options: &MedicOptions, issuer: &str, network: &str) -> MedicCheck {
    match auth::renewal_medic_summary(options.deployment_name(), issuer, network, &options.icp) {
        Ok(summary) => auth_renewal_medic_check_from_summary(summary),
        Err(err) => auth_renewal_medic_error_check(err, options.deployment_name(), issuer),
    }
}

fn auth_renewal_medic_error_check(
    error: AuthCommandError,
    deployment: &str,
    issuer: &str,
) -> MedicCheck {
    let (code, next, source) = match &error {
        AuthCommandError::InvalidIssuerPrincipal { .. } => (
            "auth_renewal_issuer_invalid",
            "pass a valid issuer canister principal".to_string(),
            MedicSource::Command,
        ),
        _ => (
            "auth_renewal_drift_fail",
            format!("run canic auth renewal status {deployment} --issuer {issuer}"),
            MedicSource::AuthRenewal,
        ),
    };

    MedicCheck::fail(
        MedicCategory::Auth,
        code,
        "auth_renewal",
        error.to_string(),
        next,
        source,
    )
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
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            lines.extend(split_medic_word(word, width));
            continue;
        }

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

fn split_medic_word(word: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
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
        let network = match options.scope {
            MedicScope::Project => options.network.clone(),
            MedicScope::Deployment => Some(options.deployment_network()),
        };
        Self::with_network(options, network, checks)
    }

    fn with_network(
        options: &MedicOptions,
        network: Option<String>,
        checks: Vec<MedicCheck>,
    ) -> Self {
        let status = aggregate_status(&checks);
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
