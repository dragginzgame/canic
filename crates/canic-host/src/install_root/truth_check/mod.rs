use super::config_selection::resolve_install_config_path;
use super::current_execution::current_install_execution_context;
use super::{capabilities::CURRENT_INSTALL_REQUIRED_CAPABILITIES, options::InstallRootOptions};
use crate::canister_build::CanisterBuildProfile;
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentExecutionPreflightV1,
    DeploymentPlanV1, LocalDeploymentCheckRequest, LocalInventoryRequest,
    check_local_deployment_at_root, collect_local_deployment_inventory_at_root,
    compare_plan_to_inventory, deployment_execution_preflight_from_check, safety_report_from_diff,
    validate_deployment_execution_preflight_for_check,
};
use crate::release_set::{AppConfigSnapshot, icp_root, workspace_root};
use canic_core::ids::FleetName;
use std::path::{Path, PathBuf};

struct CurrentInstallTruthInputs {
    workspace_root: PathBuf,
    icp_root: PathBuf,
    config_path: PathBuf,
    fleet_name: String,
}

pub(super) struct CurrentInstallTruthScope<'a> {
    workspace_root: &'a Path,
    icp_root: &'a Path,
    config_path: &'a Path,
    fleet_name: &'a str,
    observed_at: String,
    artifact_root: &'a Path,
}

impl<'a> CurrentInstallTruthScope<'a> {
    pub(super) const fn new(
        workspace_root: &'a Path,
        icp_root: &'a Path,
        config_path: &'a Path,
        fleet_name: &'a str,
        observed_at: String,
        artifact_root: &'a Path,
    ) -> Self {
        Self {
            workspace_root,
            icp_root,
            config_path,
            fleet_name,
            observed_at,
            artifact_root,
        }
    }
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
        &inputs.fleet_name,
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
        &inputs.fleet_name,
        observed_at.into(),
    )?;
    let execution_context = current_install_execution_context(
        &inputs.workspace_root,
        &inputs.icp_root,
        options.artifact_environment(),
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
    fleet_name: &str,
    observed_at: String,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let artifact_root =
        crate::release_set::artifact_root_path(icp_root, options.artifact_environment());
    current_install_deployment_truth_check_at_with_plan(
        options,
        CurrentInstallTruthScope::new(
            workspace_root,
            icp_root,
            config_path,
            fleet_name,
            observed_at,
            &artifact_root,
        ),
        None,
    )
}

pub(super) fn current_install_deployment_truth_check_at_with_plan(
    options: &InstallRootOptions,
    scope: CurrentInstallTruthScope<'_>,
    prepared_plan: Option<&DeploymentPlanV1>,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let app = AppConfigSnapshot::load(scope.config_path)?
        .app_id()
        .to_string();
    if let Some(plan) = prepared_plan.or(options.deployment_plan_override.as_ref()) {
        validate_current_install_plan_override(plan, &options.environment, scope.fleet_name, &app)?;
        let mut plan = plan.clone();
        bind_admitted_fresh_fleet_plan_digest(&mut plan, options)?;
        return current_install_deployment_truth_check_for_plan(&plan, scope, &options.environment);
    }

    let build_profile = options
        .build_profile
        .unwrap_or(CanisterBuildProfile::Release)
        .target_dir_name()
        .to_string();

    let mut check = check_local_deployment_at_root(
        &LocalDeploymentCheckRequest {
            fleet_name: scope.fleet_name.to_string(),
            app,
            environment: options.environment.clone(),
            artifact_environment: options.artifact_environment().to_string(),
            workspace_root: scope.workspace_root.to_path_buf(),
            icp_root: scope.icp_root.to_path_buf(),
            config_path: Some(scope.config_path.to_path_buf()),
            observed_at: scope.observed_at,
            runtime_variant: options.environment.clone(),
            build_profile,
        },
        scope.artifact_root,
    )?;
    bind_admitted_fresh_fleet_plan_digest(&mut check.plan, options)?;
    Ok(check)
}

fn bind_admitted_fresh_fleet_plan_digest(
    plan: &mut DeploymentPlanV1,
    options: &InstallRootOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(admitted) = options.admitted_fresh_fleet_plan_digest.as_ref() else {
        return Ok(());
    };
    if plan
        .plan_digest
        .as_ref()
        .is_some_and(|observed| observed != admitted)
    {
        return Err("deployment-truth plan digest differs from admitted fresh-Fleet plan".into());
    }
    plan.plan_digest = Some(admitted.clone());
    Ok(())
}

pub(super) fn validate_expected_app_id(
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
        "install requested App {expected}, but {} declares [app].name = {actual:?}",
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
    let config_path = if let Some(path) = options.config_path.as_deref() {
        resolve_install_config_path(&icp_root, Some(path), options.interactive_config_selection)?
    } else {
        let default_config = options
            .expected_app
            .as_ref()
            .map(|app| format!("apps/{app}/canic.toml"));
        resolve_install_config_path(
            &icp_root,
            default_config.as_deref(),
            options.interactive_config_selection,
        )?
    };
    let workspace_root = workspace_root()?;
    let app_id = AppConfigSnapshot::load(&config_path)?.app_id().to_string();
    validate_expected_app_id(options.expected_app.as_deref(), &app_id, &config_path)?;
    options.fleet_name.parse::<FleetName>()?;
    Ok(CurrentInstallTruthInputs {
        workspace_root,
        icp_root,
        config_path,
        fleet_name: options.fleet_name.clone(),
    })
}

fn current_install_deployment_truth_check_for_plan(
    plan: &DeploymentPlanV1,
    scope: CurrentInstallTruthScope<'_>,
    environment: &str,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let inventory = collect_local_deployment_inventory_at_root(
        &LocalInventoryRequest {
            fleet_name: scope.fleet_name.to_string(),
            environment: environment.to_string(),
            artifact_environment: environment.to_string(),
            workspace_root: scope.workspace_root.to_path_buf(),
            icp_root: scope.icp_root.to_path_buf(),
            config_path: Some(scope.config_path.to_path_buf()),
            observed_at: scope.observed_at,
        },
        scope.artifact_root,
    )?;
    let diff = compare_plan_to_inventory(plan, &inventory);
    let report = safety_report_from_diff(
        format!("local:{environment}:{}:report", scope.fleet_name),
        Some(format!("local:{environment}:{}:diff", scope.fleet_name)),
        &diff,
    );

    Ok(DeploymentCheckV1 {
        schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: format!("local:{environment}:{}:check", scope.fleet_name),
        plan: plan.clone(),
        inventory,
        diff,
        report,
    })
}

fn validate_current_install_plan_override(
    plan: &DeploymentPlanV1,
    environment: &str,
    fleet_name: &str,
    app: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.schema_version != crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(format!(
            "deployment plan schema mismatch: expected {}, found {}",
            crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan.schema_version
        )
        .into());
    }
    if plan.deployment_identity.environment != environment {
        return Err(format!(
            "deployment plan environment mismatch: install environment {environment}, plan environment {}",
            plan.deployment_identity.environment
        )
        .into());
    }
    if plan.deployment_identity.fleet_name != fleet_name {
        return Err(format!(
            "deployment plan Fleet mismatch: install Fleet {fleet_name}, plan Fleet {}",
            plan.deployment_identity.fleet_name
        )
        .into());
    }
    if plan.deployment_identity.app != app {
        return Err(format!(
            "deployment plan App mismatch: install App {app}, plan App {}",
            plan.deployment_identity.app
        )
        .into());
    }
    Ok(())
}
