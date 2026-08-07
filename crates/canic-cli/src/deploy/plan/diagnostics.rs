//! Module: canic_cli::deploy::plan::diagnostics
//!
//! Responsibility: classify deployment-plan blockers, warnings, and assumptions.
//! Does not own: verified evidence, comparison, proposed operations, or rendering.
//! Boundary: maps unresolved plan inputs into stable report diagnostics.

use crate::deploy::plan::{
    ASSUMPTION_KEY_LOCAL_CONFIG_POOLS, ASSUMPTION_PREFIX_FLEET_CATALOG,
    ASSUMPTION_PREFIX_LOCAL_ARTIFACTS, ASSUMPTION_PREFIX_LOCAL_CONFIG,
    ASSUMPTION_PREFIX_UNSUPPORTED,
    command::DeployPlanOptions,
    report::{
        CATEGORY_ARTIFACT, CATEGORY_CONFIG, CATEGORY_DEPLOYMENT_IDENTITY, CATEGORY_OBSERVATION,
        CATEGORY_TOPOLOGY, CATEGORY_UNSUPPORTED_SHAPE, PlanDiagnostic, PlanDiagnosticCategory,
        SEVERITY_BLOCKED, SEVERITY_UNSUPPORTED, SEVERITY_WARNING, SOURCE_CLI_ARG,
        SOURCE_DEPLOYMENT_CONFIG, SOURCE_DEPLOYMENT_PLAN_BUILDER, SOURCE_FLEET_CATALOG,
    },
};
use std::path::Path;

use canic_host::{
    deployment_truth::{DeploymentAssumptionKindV1, DeploymentAssumptionV1, DeploymentPlanV1},
    release_set::read_app_config_identity,
};

pub(super) fn target_resolution_blockers(
    options: &DeployPlanOptions,
    config_path: &Path,
) -> Vec<PlanDiagnostic> {
    if let Err(err) = validate_fleet_name(&options.fleet) {
        return vec![PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "fleet_name_invalid".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.fleet.clone(),
            detail: err,
            next: Some("use a canonical Fleet name".to_string()),
            source: SOURCE_CLI_ARG,
        }];
    }

    match read_app_config_identity(config_path) {
        Ok(app) if app == options.app => Vec::new(),
        Ok(app) => vec![PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "app_identity_mismatch".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.app.clone(),
            detail: format!(
                "{} declares App {app}, not requested App {}",
                config_path.display(),
                options.app
            ),
            next: Some("select the matching --app or --config".to_string()),
            source: SOURCE_DEPLOYMENT_CONFIG,
        }],
        Err(err) => vec![PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "app_unresolved".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.app.clone(),
            detail: format!(
                "App {} could not be resolved from {}: {err}",
                options.app,
                config_path.display()
            ),
            next: Some("provide --config with a readable config for the requested App".to_string()),
            source: SOURCE_DEPLOYMENT_CONFIG,
        }],
    }
}

fn validate_fleet_name(name: &str) -> Result<(), String> {
    name.parse::<canic_core::ids::FleetName>()
        .map(|_| ())
        .map_err(|error| format!("invalid Fleet name {name:?}: {error}"))
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
}

fn is_warning_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_FLEET_CATALOG)
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
    } else {
        "repair the local App config before planning apply".to_string()
    }
}

pub(super) fn plan_warnings(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| is_warning_plan_assumption(&assumption.key))
        .map(|assumption| PlanDiagnostic {
            category: CATEGORY_OBSERVATION,
            code: fleet_catalog_warning_code(assumption),
            severity: SEVERITY_WARNING,
            subject: plan.deployment_identity.fleet_name.clone(),
            detail: assumption.description.clone(),
            next: Some(
                "run canic deploy check after installation or provide saved evidence".to_string(),
            ),
            source: SOURCE_FLEET_CATALOG,
        })
        .collect()
}

fn fleet_catalog_warning_code(assumption: &DeploymentAssumptionV1) -> String {
    if assumption.has_kind(DeploymentAssumptionKindV1::FleetCatalogMissing)
        || assumption.has_kind(DeploymentAssumptionKindV1::FleetCatalogReadFailed)
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
    } else if key.starts_with(ASSUMPTION_PREFIX_FLEET_CATALOG) {
        CATEGORY_OBSERVATION
    } else if key == ASSUMPTION_KEY_LOCAL_CONFIG_POOLS {
        CATEGORY_TOPOLOGY
    } else {
        CATEGORY_CONFIG
    }
}

fn assumption_next(key: &str) -> Option<String> {
    if key.starts_with(ASSUMPTION_PREFIX_LOCAL_ARTIFACTS) {
        Some("run canic build or provide a build profile with resolved artifacts".to_string())
    } else if key.starts_with(ASSUMPTION_PREFIX_FLEET_CATALOG) {
        Some(
            "compare after the first Fleet install or provide deployment-check evidence"
                .to_string(),
        )
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
