//! Module: canic_cli::medic
//!
//! Responsibility: diagnose Canic project and installed-deployment readiness.
//! Does not own: deployment mutation, recovery, install-state persistence, or
//! canister control-plane changes.
//! Boundary: reads local project/deployment state and renders diagnostic-only
//! medic reports.

mod command;
mod package;
mod render;
mod report;
#[cfg(test)]
mod tests;

use crate::{
    auth::{self, AuthCommandError, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    blob_storage::{
        self, BlobStorageCommandError, BlobStorageMedicStatus, BlobStorageMedicSummary,
    },
    cli::defaults::local_network,
    support::candid::role_candid_path,
};
use canic_core::{
    bootstrap::{CanicFeatureRequirement, parse_config_model, role_required_canic_features},
    ids::CanisterRole,
    protocol::{
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
    },
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
    state_manifest::{StateAuditStatus, build_state_audit_report},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use command::MedicOptions;
pub use command::{MedicCommandError, run};
#[cfg(test)]
use command::{medic_subcommand_help_requested, usage};
use package::{
    canic_dependency_feature_snippet, canic_package_metadata, role_package_manifest_path,
};
#[cfg(test)]
use render::{MEDIC_REPORT_WIDTH, render_medic_ci_text, render_medic_json, render_medic_text};
#[cfg(test)]
use report::aggregate_status;
use report::{MedicCategory, MedicCheck, MedicReport, MedicScope, MedicSource, MedicStatus};

const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";

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
        state_audit_project_check(),
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

fn state_audit_project_check() -> MedicCheck {
    let report = build_state_audit_report(None);
    let detail = format!(
        "state audit status {} with {} check(s)",
        report.status.label(),
        report.checks.len()
    );

    match report.status {
        StateAuditStatus::Pass => MedicCheck::pass(
            MedicCategory::Runtime,
            "state_audit_pass",
            "state_manifest",
            detail,
            "none",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::Warn => MedicCheck::warn(
            MedicCategory::Runtime,
            "state_audit_warn",
            "state_manifest",
            detail,
            "run canic state audit",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::Fail => MedicCheck::fail(
            MedicCategory::Runtime,
            "state_audit_fail",
            "state_manifest",
            detail,
            "run canic state audit and fix failing state metadata checks",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::NotEvaluated => MedicCheck::not_evaluated(
            MedicCategory::Runtime,
            "state_audit_not_evaluated",
            "state_manifest",
            detail,
            "declare state metadata, then run canic state audit",
            MedicSource::StateManifest,
        ),
    }
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
    let required_features_by_role = required_canic_features_by_role(config, &roles);

    roles
        .iter()
        .flat_map(|role| {
            let mut checks = vec![check_role_package_metadata(root, config, role, &fleet)];
            checks.extend(check_role_required_canic_features(
                root,
                config,
                role,
                required_features_by_role
                    .get(&role.role)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            ));
            if !role.attached {
                checks.push(check_declared_role_not_deployable(root, config, role));
            }
            checks
        })
        .collect()
}

fn required_canic_features_by_role(
    config: &Path,
    roles: &[ConfiguredRoleLifecycle],
) -> BTreeMap<String, Vec<CanicFeatureRequirement>> {
    let Ok(config_source) = fs::read_to_string(config) else {
        return BTreeMap::new();
    };
    let Ok(config_model) = parse_config_model(&config_source) else {
        return BTreeMap::new();
    };

    roles
        .iter()
        .map(|role| {
            let role_id = CanisterRole::owned(role.role.clone());
            (
                role.role.clone(),
                role_required_canic_features(&config_model, &role_id),
            )
        })
        .filter(|(_, requirements)| !requirements.is_empty())
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

fn check_role_required_canic_features(
    root: &Path,
    config: &Path,
    role: &ConfiguredRoleLifecycle,
    requirements: &[CanicFeatureRequirement],
) -> Vec<MedicCheck> {
    if requirements.is_empty() {
        return Vec::new();
    }

    let manifest = role_package_manifest_path(config, &role.package);
    let Ok(metadata) = canic_package_metadata(&manifest) else {
        return Vec::new();
    };

    requirements
        .iter()
        .map(|requirement| {
            if metadata.canic_features.contains(requirement.feature) {
                MedicCheck::pass(
                    MedicCategory::ProjectConfig,
                    "role_required_canic_feature_present",
                    role.display.clone(),
                    format!(
                        "{} resolves required canic feature `{}` for {}",
                        display_medic_path(root, &manifest),
                        requirement.feature,
                        requirement.config_key
                    ),
                    "none",
                    MedicSource::FleetConfig,
                )
            } else {
                MedicCheck::fail(
                    MedicCategory::ProjectConfig,
                    "role_required_canic_feature_missing",
                    role.display.clone(),
                    format!(
                        "{} requires canic feature `{}` because {} is enabled ({})",
                        role.display,
                        requirement.feature,
                        requirement.config_key,
                        requirement.reason
                    ),
                    missing_canic_feature_next_action(root, &manifest, requirement.feature),
                    MedicSource::FleetConfig,
                )
            }
        })
        .collect()
}

fn missing_canic_feature_next_action(root: &Path, manifest: &Path, feature: &str) -> String {
    format!(
        "edit runtime [dependencies].canic in {} (not [build-dependencies]) and add `{feature}`; for workspace inheritance, edit inherited [workspace.dependencies].canic features; example:\n{}",
        display_medic_path(root, manifest),
        canic_dependency_feature_snippet([feature])
    )
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

fn display_medic_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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
        receipt.operation_status.label(),
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
        runtime_inspection_next(resolution),
        source,
    )
}

fn deploy_plan_next(deployment: &str) -> String {
    format!("run canic deploy plan {deployment} to inspect desired deployment shape")
}

fn runtime_inspection_next(resolution: &InstalledDeploymentResolution) -> String {
    let deployment = &resolution.state.deployment_name;
    let mut roles = resolution
        .topology
        .roles_by_canister
        .values()
        .cloned()
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();

    if let Some(role) = roles
        .iter()
        .find(|role| role.as_str() == "root")
        .or_else(|| roles.first())
    {
        return format!(
            "run canic inspect deployment {deployment} --role {role} to inspect runtime-observed status for one explicit role"
        );
    }

    let mut canisters = resolution
        .registry
        .entries
        .iter()
        .map(|entry| entry.pid.clone())
        .collect::<Vec<_>>();
    canisters.sort();
    canisters.dedup();

    canisters.first().map_or_else(
        || "none".to_string(),
        |canister| {
            format!(
                "run canic inspect canister {canister} to inspect runtime-observed status for one explicit canister"
            )
        },
    )
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
