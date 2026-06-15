use super::super::super::*;
use super::super::digest::lifecycle_authority_report_digest;
use super::super::error::LifecycleAuthorityReportError;
use super::policy::{
    external_lifecycle_controllers, lifecycle_blockers, lifecycle_consent_requirements,
    lifecycle_external_action_required, lifecycle_mode, lifecycle_reason, lifecycle_upgrade_modes,
    lifecycle_verification_requirements, lifecycle_warnings, required_lifecycle_controllers,
    sorted_unique,
};
use super::validation::{
    ensure_lifecycle_authority_report_field, ensure_unique_authority_subjects,
};
use std::collections::BTreeSet;

/// Project the existing deployment truth control classifications into the 0.45
/// lifecycle-authority view. This is observational and must not mutate IC or
/// local deployment state.
#[must_use]
pub fn lifecycle_authority_report_from_check(
    report_id: impl Into<String>,
    check: &DeploymentCheckV1,
) -> LifecycleAuthorityReportV1 {
    let mut authorities = Vec::new();
    let mut seen_subjects = BTreeSet::new();

    for expected in &check.plan.expected_canisters {
        let observed = observed_canister_for_expected(&check.inventory, expected);
        let authority = lifecycle_authority_for_expected_canister(&check.plan, expected, observed);
        seen_subjects.insert(authority.subject.clone());
        authorities.push(authority);
    }

    for expected in &check.plan.expected_pool {
        let observed = observed_pool_for_expected(&check.inventory, expected);
        let authority = lifecycle_authority_for_expected_pool(expected, observed);
        seen_subjects.insert(authority.subject.clone());
        authorities.push(authority);
    }

    for observed in &check.inventory.observed_canisters {
        let subject = lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref());
        if seen_subjects.contains(&subject) {
            continue;
        }
        authorities.push(lifecycle_authority_for_unplanned_canister(observed));
    }

    for observed in &check.inventory.observed_pool {
        let subject = lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref());
        if seen_subjects.contains(&subject) {
            continue;
        }
        authorities.push(lifecycle_authority_for_unplanned_pool(observed));
    }

    authorities.sort_by(|left, right| left.subject.cmp(&right.subject));
    let external_action_required_count = authorities
        .iter()
        .filter(|authority| authority.external_action_required)
        .count();
    let blocked_count = authorities
        .iter()
        .filter(|authority| authority.blocked)
        .count();

    let mut report = LifecycleAuthorityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        check_id: check.check_id.clone(),
        plan_id: check.plan.plan_id.clone(),
        inventory_id: check.inventory.inventory_id.clone(),
        authorities,
        external_action_required_count,
        blocked_count,
    };
    report.report_digest = lifecycle_authority_report_digest(&report);
    report
}

/// Validate archived lifecycle authority report consistency and digests.
pub fn validate_lifecycle_authority_report(
    report: &LifecycleAuthorityReportV1,
) -> Result<(), LifecycleAuthorityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(LifecycleAuthorityReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_lifecycle_authority_report_field("report_id", report.report_id.as_str())?;
    ensure_lifecycle_authority_report_field("report_digest", report.report_digest.as_str())?;
    ensure_lifecycle_authority_report_field("check_id", report.check_id.as_str())?;
    ensure_lifecycle_authority_report_field("plan_id", report.plan_id.as_str())?;
    ensure_lifecycle_authority_report_field("inventory_id", report.inventory_id.as_str())?;
    ensure_unique_authority_subjects(&report.authorities)?;
    if report.external_action_required_count
        != report
            .authorities
            .iter()
            .filter(|authority| authority.external_action_required)
            .count()
        || report.blocked_count
            != report
                .authorities
                .iter()
                .filter(|authority| authority.blocked)
                .count()
    {
        return Err(LifecycleAuthorityReportError::CountMismatch);
    }
    if report.report_digest != lifecycle_authority_report_digest(report) {
        return Err(LifecycleAuthorityReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

fn lifecycle_authority_for_expected_canister(
    plan: &DeploymentPlanV1,
    expected: &ExpectedCanisterV1,
    observed: Option<&ObservedCanisterV1>,
) -> LifecycleAuthorityV1 {
    let canister_id = expected
        .canister_id
        .clone()
        .or_else(|| observed.map(|observed| observed.canister_id.clone()));
    let role = Some(expected.role.clone());
    let control_class = observed.map_or(expected.control_class, |observed| observed.control_class);
    let observed_controllers =
        observed.map_or_else(Vec::new, |observed| observed.controllers.clone());
    lifecycle_authority(
        lifecycle_subject_for_parts(canister_id.as_deref(), role.as_deref()),
        canister_id,
        role,
        control_class,
        observed_controllers,
        &plan.authority_profile.expected_controllers,
        plan.expected_verifier_readiness.required,
    )
}

fn lifecycle_authority_for_expected_pool(
    expected: &ExpectedPoolCanisterV1,
    observed: Option<&ObservedPoolCanisterV1>,
) -> LifecycleAuthorityV1 {
    let canister_id = expected
        .canister_id
        .clone()
        .or_else(|| observed.map(|observed| observed.canister_id.clone()));
    let role = expected
        .role
        .clone()
        .or_else(|| observed.and_then(|observed| observed.role.clone()));
    let control_class = observed.map_or(CanisterControlClassV1::CanicManagedPool, |observed| {
        observed.control_class
    });
    lifecycle_authority(
        lifecycle_subject_for_parts(canister_id.as_deref(), role.as_deref()),
        canister_id,
        role,
        control_class,
        Vec::new(),
        &[],
        false,
    )
}

fn lifecycle_authority_for_unplanned_canister(
    observed: &ObservedCanisterV1,
) -> LifecycleAuthorityV1 {
    lifecycle_authority(
        lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref()),
        Some(observed.canister_id.clone()),
        observed.role.clone(),
        observed.control_class,
        observed.controllers.clone(),
        &[],
        false,
    )
}

fn lifecycle_authority_for_unplanned_pool(
    observed: &ObservedPoolCanisterV1,
) -> LifecycleAuthorityV1 {
    lifecycle_authority(
        lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref()),
        Some(observed.canister_id.clone()),
        observed.role.clone(),
        observed.control_class,
        Vec::new(),
        &[],
        false,
    )
}

fn lifecycle_authority(
    subject: String,
    canister_id: Option<String>,
    role: Option<String>,
    control_class: CanisterControlClassV1,
    observed_controllers: Vec<String>,
    expected_controllers: &[String],
    verifier_required: bool,
) -> LifecycleAuthorityV1 {
    let required_controllers = required_lifecycle_controllers(control_class, expected_controllers);
    let external_controllers =
        external_lifecycle_controllers(control_class, &observed_controllers, &required_controllers);
    let consent_requirements = lifecycle_consent_requirements(control_class, &external_controllers);
    let allowed_upgrade_modes = lifecycle_upgrade_modes(control_class);
    let verification_requirements = lifecycle_verification_requirements(verifier_required);
    let external_action_required = lifecycle_external_action_required(control_class);
    let blocked = control_class == CanisterControlClassV1::UnknownUnsafe;
    let lifecycle_mode = lifecycle_mode(control_class);
    let blockers = lifecycle_blockers(control_class);
    let warnings = lifecycle_warnings(control_class);
    let reason = lifecycle_reason(control_class);
    LifecycleAuthorityV1 {
        subject,
        canister_id,
        role,
        control_class,
        lifecycle_mode,
        observed_controllers,
        expected_deployment_controllers: sorted_unique(expected_controllers.to_vec()),
        external_controllers,
        required_controllers,
        consent_requirements,
        allowed_upgrade_modes,
        verification_requirements,
        external_action_required,
        blocked,
        blockers,
        warnings,
        reason,
    }
}

fn observed_canister_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedCanisterV1,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(canister_id) = &expected.canister_id
        && let Some(observed) = inventory
            .observed_canisters
            .iter()
            .find(|observed| &observed.canister_id == canister_id)
    {
        return Some(observed);
    }
    inventory
        .observed_canisters
        .iter()
        .find(|observed| observed.role.as_deref() == Some(expected.role.as_str()))
}

fn observed_pool_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedPoolCanisterV1,
) -> Option<&'a ObservedPoolCanisterV1> {
    if let Some(canister_id) = &expected.canister_id
        && let Some(observed) = inventory
            .observed_pool
            .iter()
            .find(|observed| &observed.canister_id == canister_id)
    {
        return Some(observed);
    }
    inventory.observed_pool.iter().find(|observed| {
        observed.pool == expected.pool && observed.role.as_deref() == expected.role.as_deref()
    })
}

fn lifecycle_subject(canister_id: &str, role: Option<&str>) -> String {
    lifecycle_subject_for_parts(Some(canister_id), role)
}

fn lifecycle_subject_for_parts(canister_id: Option<&str>, role: Option<&str>) -> String {
    match (role, canister_id) {
        (Some(role), Some(canister_id)) => format!("{role}:{canister_id}"),
        (Some(role), None) => format!("{role}:unassigned"),
        (None, Some(canister_id)) => canister_id.to_string(),
        (None, None) => "unknown".to_string(),
    }
}
