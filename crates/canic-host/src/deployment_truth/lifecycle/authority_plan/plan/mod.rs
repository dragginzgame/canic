use super::super::super::*;
use super::super::digest::external_lifecycle_plan_digest;
use super::super::error::ExternalLifecyclePlanError;
use super::authority::lifecycle_authority_report_from_check;
use super::policy::required_external_action;
use super::validation::{
    ensure_external_lifecycle_plan_field, ensure_unique_lifecycle_subjects,
    ensure_unique_role_upgrade_subjects,
};

/// Build the central 0.45 lifecycle plan from deployment truth.
///
/// This partitions roles into directly executable, externally proposed, and
/// blocked lifecycle rows. It is passive and does not perform proposal
/// delivery, consent, or execution.
#[must_use]
pub fn external_lifecycle_plan_from_check(
    lifecycle_plan_id: impl Into<String>,
    lifecycle_authority_report_id: impl Into<String>,
    check: &DeploymentCheckV1,
) -> ExternalLifecyclePlanV1 {
    let lifecycle_authority_report =
        lifecycle_authority_report_from_check(lifecycle_authority_report_id, check);
    let lifecycle_authority_rows = lifecycle_authority_report.authorities;
    let directly_executable_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| {
            authority.lifecycle_mode == LifecycleModeV1::DirectDeploymentAuthority
                && !authority.blocked
        })
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let proposed_external_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| authority.external_action_required && !authority.blocked)
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let blocked_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| authority.blocked)
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let residual_exposure = proposed_external_role_upgrades
        .iter()
        .map(|upgrade| {
            format!(
                "{} remains pending external lifecycle action",
                upgrade.subject
            )
        })
        .collect::<Vec<_>>();
    let status = if !blocked_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::Blocked
    } else if !proposed_external_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    } else {
        ExternalLifecyclePlanStatusV1::Ready
    };
    let deployment_plan_digest = stable_json_sha256_hex(&check.plan);
    let mut plan = ExternalLifecyclePlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        lifecycle_plan_id: lifecycle_plan_id.into(),
        lifecycle_plan_digest: String::new(),
        lifecycle_authority_report_id: lifecycle_authority_report.report_id,
        deployment_plan_id: check.plan.plan_id.clone(),
        deployment_plan_digest,
        inventory_id: check.inventory.inventory_id.clone(),
        lifecycle_authority_rows,
        directly_executable_role_upgrades,
        proposed_external_role_upgrades,
        blocked_role_upgrades,
        dependency_blockers: Vec::new(),
        protected_call_implications: protected_call_implications_for_check(check),
        residual_exposure,
        status,
    };
    plan.lifecycle_plan_digest = external_lifecycle_plan_digest(&plan);
    plan
}

/// Validate archived external lifecycle plan consistency and digests.
pub fn validate_external_lifecycle_plan(
    plan: &ExternalLifecyclePlanV1,
) -> Result<(), ExternalLifecyclePlanError> {
    if plan.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecyclePlanError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: plan.schema_version,
        });
    }
    ensure_external_lifecycle_plan_field("lifecycle_plan_id", plan.lifecycle_plan_id.as_str())?;
    ensure_external_lifecycle_plan_field(
        "lifecycle_authority_report_id",
        plan.lifecycle_authority_report_id.as_str(),
    )?;
    ensure_external_lifecycle_plan_field("deployment_plan_id", plan.deployment_plan_id.as_str())?;
    ensure_external_lifecycle_plan_field("inventory_id", plan.inventory_id.as_str())?;
    if plan.lifecycle_plan_digest != external_lifecycle_plan_digest(plan) {
        return Err(ExternalLifecyclePlanError::DigestMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if plan.status != expected_lifecycle_plan_status(plan) {
        return Err(ExternalLifecyclePlanError::StatusMismatch);
    }
    ensure_unique_lifecycle_subjects(&plan.lifecycle_authority_rows)?;
    ensure_unique_role_upgrade_subjects(&plan.directly_executable_role_upgrades)?;
    ensure_unique_role_upgrade_subjects(&plan.proposed_external_role_upgrades)?;
    ensure_unique_role_upgrade_subjects(&plan.blocked_role_upgrades)?;
    Ok(())
}

/// Validate that an archived external lifecycle plan still matches its source
/// deployment truth check.
pub fn validate_external_lifecycle_plan_for_check(
    plan: &ExternalLifecyclePlanV1,
    check: &DeploymentCheckV1,
) -> Result<(), ExternalLifecyclePlanError> {
    validate_external_lifecycle_plan(plan)?;
    let expected = external_lifecycle_plan_from_check(
        plan.lifecycle_plan_id.clone(),
        plan.lifecycle_authority_report_id.clone(),
        check,
    );
    if plan != &expected {
        return Err(ExternalLifecyclePlanError::SourceMismatch {
            field: "deployment_check",
        });
    }
    Ok(())
}

fn external_lifecycle_role_upgrade(
    authority: &LifecycleAuthorityV1,
) -> ExternalLifecycleRoleUpgradeV1 {
    ExternalLifecycleRoleUpgradeV1 {
        subject: authority.subject.clone(),
        canister_id: authority.canister_id.clone(),
        role: authority.role.clone(),
        control_class: authority.control_class,
        lifecycle_mode: authority.lifecycle_mode,
        required_external_action: authority
            .external_action_required
            .then(|| required_external_action(authority.lifecycle_mode).to_string()),
        blockers: authority.blockers.clone(),
        warnings: authority.warnings.clone(),
    }
}

fn protected_call_implications_for_check(check: &DeploymentCheckV1) -> Vec<String> {
    if check.plan.expected_verifier_readiness.required {
        vec!["protected-call verifier readiness must be checked before completion".to_string()]
    } else {
        Vec::new()
    }
}

const fn expected_lifecycle_plan_status(
    plan: &ExternalLifecyclePlanV1,
) -> ExternalLifecyclePlanStatusV1 {
    if !plan.blocked_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::Blocked
    } else if !plan.proposed_external_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    } else {
        ExternalLifecyclePlanStatusV1::Ready
    }
}
