//! Module: canic_cli::medic
//!
//! Responsibility: diagnose Canic project and installed-deployment readiness.
//! Does not own: deployment mutation, recovery, install-state persistence, or
//! canister control-plane changes.
//! Boundary: reads local project/deployment state and renders diagnostic-only
//! medic reports.

mod auth;
mod blob_storage;
mod command;
mod package;
mod project;
mod render;
mod report;
mod role_contract;
#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::{
    auth::{AuthCommandError, AuthRenewalMedicStatus, AuthRenewalMedicSummary},
    blob_storage::{BlobStorageCommandError, BlobStorageMedicStatus, BlobStorageMedicSummary},
};
use crate::{cli::defaults::local_network, support::candid::role_candid_path};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
use canic_core::ids::CanisterRole;
use canic_core::role_contract::RoleContractFinding;
#[cfg(test)]
use canic_host::icp::local_canister_candid_path;
#[cfg(test)]
use canic_host::state_manifest::{StateAuditStatus, build_state_audit_report};
use canic_host::{
    canister_ready::query_canister_ready,
    deployment_truth::{
        DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    },
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    install_root::{
        InstallState, discover_project_canic_config_choices,
        latest_deployment_truth_receipt_path_from_root,
    },
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        InstalledDeploymentSource, read_installed_deployment_state_from_root,
        resolve_installed_deployment_from_root,
    },
    release_set::{configured_fleet_name, configured_role_lifecycle},
    state_manifest::{StateManifestResolution, resolve_project_state_manifest},
};

use auth::check_auth_renewal;
#[cfg(test)]
use auth::{auth_renewal_medic_check_from_summary, auth_renewal_medic_error_check};
#[cfg(test)]
use blob_storage::{
    blob_storage_billing_roles_from_candid_dir, blob_storage_medic_check_from_summary,
    blob_storage_medic_error_check, candid_declares_blob_storage_billing,
};
use blob_storage::{check_blob_storage_billing, check_blob_storage_not_selected};
use command::MedicOptions;
pub use command::{MedicCommandError, run};
#[cfg(test)]
use command::{medic_subcommand_help_requested, usage};
#[cfg(test)]
use project::project_network_selection_check;
use project::{project_config_checks, state_audit_project_check};
#[cfg(test)]
use render::{MEDIC_REPORT_WIDTH, render_medic_ci_text, render_medic_json, render_medic_text};
#[cfg(test)]
use report::aggregate_status;
use report::{MedicCategory, MedicCheck, MedicReport, MedicScope, MedicSource, MedicStatus};
#[cfg(test)]
use role_contract::project_config_quality_checks;

const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";
const DEPLOYMENT_NOT_SELECTED_CHECK_CODE: &str = "deployment_not_selected";

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
            let state_resolution = match discover_project_canic_config_choices(&root) {
                Ok(configs) => resolve_project_state_manifest(&root, &configs, None),
                Err(error) => StateManifestResolution::Rejected {
                    errors: vec![RoleContractFinding::DependencyShapeUnsupported {
                        reason: error.to_string(),
                    }],
                },
            };
            checks.push(state_audit_project_check(&state_resolution));
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
            checks.push(MedicCheck::not_evaluated(
                MedicCategory::Runtime,
                "state_audit_not_evaluated",
                "state_manifest",
                "state audit requires a resolved Canic project root",
                "run from a Canic project root, then run canic state audit",
                MedicSource::StateManifest,
            ));
        }
    }

    checks.push(MedicCheck::not_evaluated(
        MedicCategory::DeploymentState,
        DEPLOYMENT_NOT_SELECTED_CHECK_CODE,
        "deployment",
        "no deployment target was selected",
        "run canic medic deployment <deployment>",
        MedicSource::Command,
    ));
    checks
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
        .filter(|check| check.code != DEPLOYMENT_NOT_SELECTED_CHECK_CODE)
        .collect::<Vec<_>>();
    let network = &context.network;
    let icp_root = context.icp_root.as_deref();

    checks.push(context.network_check.clone());

    let state_result = match icp_root {
        Some(root) => {
            read_installed_deployment_state_from_root(network, options.deployment_name(), root)
                .map_err(Some)
        }
        None => Err(None),
    };
    let state = match state_result {
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
        Err(Some(InstalledDeploymentError::NoInstalledDeployment { .. })) => {
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
            let detail = err.map_or_else(
                || "could not resolve ICP project root".to_string(),
                |err| err.to_string(),
            );
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                detail,
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
