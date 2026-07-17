use super::config_selection::resolve_install_config_path;
use super::current_execution::current_install_execution_context;
use super::state::{read_named_deployment_install_state_from_root, validate_state_name};
use super::{capabilities::CURRENT_INSTALL_REQUIRED_CAPABILITIES, options::InstallRootOptions};
use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentExecutionPreflightV1,
    DeploymentPlanV1, LocalDeploymentCheckRequest, LocalInventoryRequest, check_local_deployment,
    collect_local_deployment_inventory, compare_plan_to_inventory,
    deployment_execution_preflight_from_check, safety_report_from_diff,
    validate_deployment_execution_preflight_for_check,
};
use crate::release_set::{configured_fleet_name, icp_root, workspace_root};
use std::path::{Path, PathBuf};

struct CurrentInstallTruthInputs {
    workspace_root: PathBuf,
    icp_root: PathBuf,
    config_path: PathBuf,
    deployment_name: String,
}

/// Build the same read-only deployment truth check that can be used as a
/// preflight for the current install inputs without mutating deployment state.
pub fn check_install_deployment_truth(
    options: &InstallRootOptions,
    observed_at: impl Into<String>,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let inputs = resolve_current_install_truth_inputs(options)?;
    current_install_deployment_truth_check_at(
        options,
        &inputs.workspace_root,
        &inputs.icp_root,
        &inputs.config_path,
        &inputs.deployment_name,
        observed_at.into(),
    )
}

/// Build a read-only execution preflight for the current install inputs.
///
/// This validates the current plan, safety report, authority reconciliation,
/// and executor capabilities without opening the mutating install path or
/// writing local receipt state.
pub fn check_install_execution_preflight(
    options: &InstallRootOptions,
    observed_at: impl Into<String>,
) -> Result<DeploymentExecutionPreflightV1, Box<dyn std::error::Error>> {
    let inputs = resolve_current_install_truth_inputs(options)?;
    let check = current_install_deployment_truth_check_at(
        options,
        &inputs.workspace_root,
        &inputs.icp_root,
        &inputs.config_path,
        &inputs.deployment_name,
        observed_at.into(),
    )?;
    let execution_context = current_install_execution_context(
        &inputs.workspace_root,
        &inputs.icp_root,
        &options.network,
    );
    let executor = CurrentCliDeploymentExecutor::new(
        execution_context.workspace_root,
        execution_context.icp_root,
        execution_context.artifact_roots,
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    );
    validate_deployment_execution_preflight_for_check(&check, &preflight)?;
    Ok(preflight)
}

pub(super) fn current_install_deployment_truth_check_at(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    observed_at: String,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    if let Some(plan) = &options.deployment_plan_override {
        validate_current_install_plan_override(plan, &options.network, deployment_name)?;
        return current_install_deployment_truth_check_for_plan(
            plan,
            workspace_root,
            icp_root,
            config_path,
            deployment_name,
            observed_at,
            &options.network,
        );
    }

    let build_profile = options
        .build_profile
        .unwrap_or(CanisterBuildProfile::Release)
        .target_dir_name()
        .to_string();

    check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: deployment_name.to_string(),
        network: options.network.clone(),
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: Some(config_path.to_path_buf()),
        observed_at,
        runtime_variant: options.network.clone(),
        build_profile,
    })
    .map_err(Into::into)
}

pub(super) fn validate_expected_fleet_name(
    expected: Option<&str>,
    actual: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if expected == actual {
        return Ok(());
    }
    Err(format!(
        "install requested fleet {expected}, but {} declares [fleet].name = {actual:?}",
        config_path.display()
    )
    .into())
}

fn resolve_current_install_truth_inputs(
    options: &InstallRootOptions,
) -> Result<CurrentInstallTruthInputs, Box<dyn std::error::Error>> {
    let icp_root = match &options.icp_root {
        Some(path) => path.canonicalize()?,
        None => icp_root()?,
    };
    let state = match options.deployment_name.as_deref() {
        Some(deployment) => {
            read_named_deployment_install_state_from_root(&icp_root, &options.network, deployment)?
        }
        None => None,
    };
    let config_path = match (options.config_path.as_deref(), state.as_ref()) {
        (Some(path), _) => resolve_install_config_path(
            &icp_root,
            Some(path),
            options.interactive_config_selection,
        )?,
        (None, Some(state)) => resolve_install_config_path(
            &icp_root,
            Some(&state.config_path),
            options.interactive_config_selection,
        )?,
        (None, None) => {
            let default_config = options
                .deployment_name
                .as_ref()
                .map(|deployment| default_config_path_for_deployment(deployment));
            resolve_install_config_path(
                &icp_root,
                default_config.as_deref(),
                options.interactive_config_selection,
            )?
        }
    };
    let workspace_root = workspace_root()?;
    let fleet_template = configured_fleet_name(&config_path)?;
    let expected_fleet = options
        .expected_fleet
        .as_deref()
        .or_else(|| state.as_ref().map(|state| state.fleet_template.as_str()));
    validate_expected_fleet_name(expected_fleet, &fleet_template, &config_path)?;
    validate_state_name(&fleet_template)?;
    let deployment_name = options
        .deployment_name
        .clone()
        .unwrap_or_else(|| fleet_template.clone());
    validate_state_name(&deployment_name)?;
    Ok(CurrentInstallTruthInputs {
        workspace_root,
        icp_root,
        config_path,
        deployment_name,
    })
}

fn default_config_path_for_deployment(deployment: &str) -> String {
    format!("fleets/{deployment}/canic.toml")
}

fn current_install_deployment_truth_check_for_plan(
    plan: &DeploymentPlanV1,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    observed_at: String,
    network: &str,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: deployment_name.to_string(),
        network: network.to_string(),
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: Some(config_path.to_path_buf()),
        observed_at,
    })?;
    let diff = compare_plan_to_inventory(plan, &inventory);
    let report = safety_report_from_diff(
        format!("local:{network}:{deployment_name}:report"),
        Some(format!("local:{network}:{deployment_name}:diff")),
        &diff,
    );

    Ok(DeploymentCheckV1 {
        schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: format!("local:{network}:{deployment_name}:check"),
        plan: plan.clone(),
        inventory,
        diff,
        report,
    })
}

fn validate_current_install_plan_override(
    plan: &DeploymentPlanV1,
    network: &str,
    deployment_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.schema_version != crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(format!(
            "deployment plan schema mismatch: expected {}, found {}",
            crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan.schema_version
        )
        .into());
    }
    if plan.deployment_identity.network != network {
        return Err(format!(
            "deployment plan network mismatch: install network {network}, plan network {}",
            plan.deployment_identity.network
        )
        .into());
    }
    if plan.deployment_identity.deployment_name != deployment_name {
        return Err(format!(
            "deployment plan target mismatch: install deployment {deployment_name}, plan deployment {}",
            plan.deployment_identity.deployment_name
        )
        .into());
    }
    Ok(())
}
