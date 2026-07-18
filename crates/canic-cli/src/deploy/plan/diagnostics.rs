//! Module: canic_cli::deploy::plan::diagnostics
//!
//! Responsibility: classify deployment-plan blockers, warnings, and assumptions.
//! Does not own: verified evidence, comparison, proposed operations, or rendering.
//! Boundary: maps unresolved plan inputs into stable report diagnostics.

use crate::deploy::plan::{
    ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS, ASSUMPTION_KEY_LOCAL_CONFIG_POOLS,
    ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID, ASSUMPTION_PREFIX_LOCAL_ARTIFACTS,
    ASSUMPTION_PREFIX_LOCAL_CONFIG, ASSUMPTION_PREFIX_LOCAL_STATE, ASSUMPTION_PREFIX_UNSUPPORTED,
    command::DeployPlanOptions,
    report::{
        CATEGORY_ARTIFACT, CATEGORY_AUTHORITY, CATEGORY_CONFIG, CATEGORY_DEPLOYMENT_IDENTITY,
        CATEGORY_OBSERVATION, CATEGORY_TOPOLOGY, CATEGORY_UNSUPPORTED_SHAPE, PlanDiagnostic,
        PlanDiagnosticCategory, SEVERITY_BLOCKED, SEVERITY_UNSUPPORTED, SEVERITY_WARNING,
        SOURCE_CLI_ARG, SOURCE_DEPLOYMENT_CONFIG, SOURCE_DEPLOYMENT_PLAN_BUILDER,
        SOURCE_INSTALLED_DEPLOYMENT,
    },
};
use std::path::Path;

use canic_host::{
    deployment_truth::{DeploymentAssumptionKindV1, DeploymentAssumptionV1, DeploymentPlanV1},
    release_set::configured_fleet_name,
};

pub(super) fn target_resolution_blockers(
    options: &DeployPlanOptions,
    config_path: &Path,
) -> Vec<PlanDiagnostic> {
    if let Err(err) = validate_deployment_target_name(&options.deployment) {
        return vec![PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "deployment_target_invalid".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.deployment.clone(),
            detail: err,
            next: Some("use letters, numbers, '-' or '_' for deployment target names".to_string()),
            source: SOURCE_CLI_ARG,
        }];
    }

    match configured_fleet_name(config_path) {
        Ok(_) => Vec::new(),
        Err(err) => vec![PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "deployment_target_unresolved".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.deployment.clone(),
            detail: format!(
                "deployment target {} could not be resolved from {}: {err}",
                options.deployment,
                config_path.display()
            ),
            next: Some(
                "provide --config with a readable fleet config for this deployment".to_string(),
            ),
            source: SOURCE_DEPLOYMENT_CONFIG,
        }],
    }
}

fn validate_deployment_target_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid deployment target name {name:?}; use letters, numbers, '-' or '_'"
        ))
    }
}

pub(super) fn plan_assumptions(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| !is_unsupported_plan_assumption(&assumption.key))
        .filter(|assumption| !is_blocking_plan_assumption(&assumption.key))
        .filter(|assumption| !is_warning_plan_assumption(&assumption.key))
        .map(assumption_diagnostic)
        .collect()
}

pub(super) fn plan_blockers(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| {
            is_unsupported_plan_assumption(&assumption.key)
                || is_blocking_plan_assumption(&assumption.key)
        })
        .map(blocking_assumption_diagnostic)
        .collect()
}

fn is_unsupported_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_UNSUPPORTED)
}

fn is_blocking_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_LOCAL_CONFIG)
        || key == ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID
}

fn is_warning_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_LOCAL_STATE) && !is_blocking_plan_assumption(key)
}

fn blocking_assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    let unsupported = is_unsupported_plan_assumption(&assumption.key);
    PlanDiagnostic {
        category: if unsupported {
            CATEGORY_UNSUPPORTED_SHAPE
        } else {
            assumption_category(&assumption.key)
        },
        code: diagnostic_code(&assumption.key),
        severity: if unsupported {
            SEVERITY_UNSUPPORTED
        } else {
            SEVERITY_BLOCKED
        },
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: Some(blocking_assumption_next(&assumption.key)),
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }
}

fn blocking_assumption_next(key: &str) -> String {
    if is_unsupported_plan_assumption(key) {
        "change the desired deployment shape to one supported by canic deploy plan".to_string()
    } else if key == ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID {
        "run canic deploy check and verify the registered root before planning apply".to_string()
    } else {
        "repair the local fleet config before planning apply".to_string()
    }
}

pub(super) fn plan_warnings(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| is_warning_plan_assumption(&assumption.key))
        .map(|assumption| PlanDiagnostic {
            category: CATEGORY_OBSERVATION,
            code: local_state_warning_code(assumption),
            severity: SEVERITY_WARNING,
            subject: plan.deployment_identity.deployment_name.clone(),
            detail: assumption.description.clone(),
            next: Some(
                "run canic deploy check after installation or provide saved evidence".to_string(),
            ),
            source: SOURCE_INSTALLED_DEPLOYMENT,
        })
        .collect()
}

fn local_state_warning_code(assumption: &DeploymentAssumptionV1) -> String {
    if is_observed_state_drift_assumption(assumption) {
        "observed_inventory_drift".to_string()
    } else if assumption.has_kind(DeploymentAssumptionKindV1::LocalStateMissing)
        || assumption.has_kind(DeploymentAssumptionKindV1::LocalStateReadFailed)
    {
        "observed_inventory_unavailable".to_string()
    } else {
        diagnostic_code(&assumption.key)
    }
}

fn assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    PlanDiagnostic {
        category: assumption_category(&assumption.key),
        code: diagnostic_code(&assumption.key),
        severity: SEVERITY_WARNING,
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: assumption_next(&assumption.key),
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }
}

fn assumption_category(key: &str) -> PlanDiagnosticCategory {
    if key.starts_with(ASSUMPTION_PREFIX_LOCAL_ARTIFACTS) {
        CATEGORY_ARTIFACT
    } else if key.starts_with(ASSUMPTION_PREFIX_LOCAL_STATE) {
        CATEGORY_OBSERVATION
    } else if key == ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS {
        CATEGORY_AUTHORITY
    } else if key == ASSUMPTION_KEY_LOCAL_CONFIG_POOLS {
        CATEGORY_TOPOLOGY
    } else {
        CATEGORY_CONFIG
    }
}

fn assumption_next(key: &str) -> Option<String> {
    if key.starts_with(ASSUMPTION_PREFIX_LOCAL_ARTIFACTS) {
        Some("run canic build or provide a build profile with resolved artifacts".to_string())
    } else if key.starts_with(ASSUMPTION_PREFIX_LOCAL_STATE) {
        Some("compare after first deployment or provide deployment-check evidence".to_string())
    } else {
        None
    }
}

fn diagnostic_code(key: &str) -> String {
    let mut code = String::new();
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            code.push(ch.to_ascii_lowercase());
        } else if !code.ends_with('_') {
            code.push('_');
        }
    }
    code.trim_matches('_').to_string()
}

pub(super) fn is_observed_state_drift_assumption(assumption: &DeploymentAssumptionV1) -> bool {
    assumption.has_kind(DeploymentAssumptionKindV1::LocalStateEnvironmentMismatch)
}
