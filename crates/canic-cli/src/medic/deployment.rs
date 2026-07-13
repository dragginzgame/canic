//! Module: canic_cli::medic::deployment
//!
//! Responsibility: construct installed-deployment, registry, receipt, and root checks.
//! Does not own: deployment mutation, check ordering, or report rendering.
//! Boundary: maps local and runtime deployment evidence into Medic checks.

use crate::{
    cli::defaults::local_network,
    medic::{
        command::MedicOptions,
        display_medic_path,
        report::{MedicCategory, MedicCheck, MedicSource, MedicStatus},
    },
    support::candid::role_candid_path,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

use canic_host::{
    canister_ready::query_canister_ready,
    deployment_truth::{
        DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    },
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    install_root::{
        InstallState, discover_project_canic_config_choices,
        latest_deployment_truth_receipt_path_from_root,
    },
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        InstalledDeploymentSource, resolve_installed_deployment_from_root,
    },
    release_set::{configured_fleet_name, configured_role_lifecycle},
};

///
/// DeploymentMedicContext
///

pub(super) struct DeploymentMedicContext {
    pub(super) icp_root: Option<PathBuf>,
    pub(super) network: String,
    pub(super) network_check: MedicCheck,
}

pub(super) fn deployment_medic_context(options: &MedicOptions) -> DeploymentMedicContext {
    let icp_root = resolve_current_canic_icp_root().ok();
    let (network, network_check) = deployment_network_selection(options, icp_root.as_deref());
    DeploymentMedicContext {
        icp_root,
        network,
        network_check,
    }
}

pub(super) fn deployment_network_selection(
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

pub(super) fn deployment_name_conflation_checks(root: &Path, deployment: &str) -> Vec<MedicCheck> {
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

pub(super) fn installed_deployment_state_checks(
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

pub(super) fn check_deployment_truth_receipt(
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
            "run from a Canic project root",
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
            "run from a Canic project root",
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

pub(super) fn check_deployment_registry_not_evaluated(
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

pub(super) fn deployment_registry_observed_check(
    resolution: &InstalledDeploymentResolution,
) -> MedicCheck {
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

pub(super) fn deploy_plan_then(deployment: &str, next: impl AsRef<str>) -> String {
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
        InstalledDeploymentError::Icp(_) => MedicSource::IcpCli,
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

pub(super) fn check_deployment_network(state: &InstallState, selected_network: &str) -> MedicCheck {
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

pub(super) fn check_root_canister_id(state: &InstallState) -> MedicCheck {
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

pub(super) fn check_root_readiness_not_evaluated(
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

pub(super) fn root_readiness_source(network: &str) -> MedicSource {
    if network == local_network() {
        MedicSource::LocalReplica
    } else {
        MedicSource::IcpCli
    }
}
