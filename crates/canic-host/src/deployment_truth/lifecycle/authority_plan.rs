use super::super::*;
use super::digest::{external_lifecycle_plan_digest, lifecycle_authority_report_digest};
use super::error::{ExternalLifecyclePlanError, LifecycleAuthorityReportError};
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

fn required_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    expected_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled
        | CanisterControlClassV1::JointlyControlled => sorted_unique(expected_controllers.to_vec()),
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled
        | CanisterControlClassV1::UnknownUnsafe => Vec::new(),
    }
}

fn external_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    observed_controllers: &[String],
    required_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
        CanisterControlClassV1::JointlyControlled => {
            let required = required_controllers.iter().collect::<BTreeSet<_>>();
            sorted_unique(
                observed_controllers
                    .iter()
                    .filter(|controller| !required.contains(controller))
                    .cloned()
                    .collect(),
            )
        }
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled => sorted_unique(observed_controllers.to_vec()),
    }
}

fn lifecycle_consent_requirements(
    control_class: CanisterControlClassV1,
    external_controllers: &[String],
) -> Vec<ConsentRequirementV1> {
    if !lifecycle_external_action_required(control_class) {
        return Vec::new();
    }
    vec![ConsentRequirementV1 {
        consent_subject_kind: consent_subject_kind(control_class),
        required_principals: sorted_unique(external_controllers.to_vec()),
        required_controller_set_digest: Some(stable_json_sha256_hex(&external_controllers)),
        consent_channel_kind: consent_channel_kind(control_class),
        required_action: required_consent_action(control_class),
    }]
}

const fn consent_subject_kind(control_class: CanisterControlClassV1) -> ConsentSubjectKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentSubjectKindV1::ProjectHub,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::JointlyControlled => {
            ConsentSubjectKindV1::CustomerController
        }
        CanisterControlClassV1::UserControlled => ConsentSubjectKindV1::UserPrincipal,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentSubjectKindV1::UnknownExternalController
        }
    }
}

const fn consent_channel_kind(control_class: CanisterControlClassV1) -> ConsentChannelKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentChannelKindV1::DelegatedInstall,
        CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => ConsentChannelKindV1::GeneratedCommand,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentChannelKindV1::OutOfBand
        }
    }
}

const fn required_consent_action(
    control_class: CanisterControlClassV1,
) -> ExternalUpgradeAuthorizationModeV1 {
    match control_class {
        CanisterControlClassV1::JointlyControlled => {
            ExternalUpgradeAuthorizationModeV1::ConsentForDirectInstall
        }
        CanisterControlClassV1::CanicManagedPool => {
            ExternalUpgradeAuthorizationModeV1::DelegatedInstallAuthority
        }
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ExternalUpgradeAuthorizationModeV1::ObserveAndVerifyOnly
        }
    }
}

const fn lifecycle_mode(control_class: CanisterControlClassV1) -> LifecycleModeV1 {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => LifecycleModeV1::DirectDeploymentAuthority,
        CanisterControlClassV1::CanicManagedPool => LifecycleModeV1::DelegatedInstallRequired,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            LifecycleModeV1::ExternalCompletionOnly
        }
        CanisterControlClassV1::JointlyControlled => LifecycleModeV1::ProposalRequired,
        CanisterControlClassV1::UnknownUnsafe => LifecycleModeV1::UnknownUnsafeBlocked,
    }
}

fn lifecycle_blockers(control_class: CanisterControlClassV1) -> Vec<String> {
    if control_class == CanisterControlClassV1::UnknownUnsafe {
        vec!["unknown unsafe controller state blocks lifecycle action".to_string()]
    } else {
        Vec::new()
    }
}

fn lifecycle_warnings(control_class: CanisterControlClassV1) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => {
            vec!["pool-aware lifecycle policy is required before mutation".to_string()]
        }
        CanisterControlClassV1::ExternallyImported => {
            vec!["external controller action or verification is required".to_string()]
        }
        CanisterControlClassV1::JointlyControlled => {
            vec!["joint controller consent or delegation is required".to_string()]
        }
        CanisterControlClassV1::UserControlled => {
            vec!["user or delegated lifecycle action is required".to_string()]
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
    }
}

fn lifecycle_upgrade_modes(control_class: CanisterControlClassV1) -> Vec<LifecycleUpgradeModeV1> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => vec![
            LifecycleUpgradeModeV1::DirectByDeploymentAuthority,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
        ],
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => vec![
            LifecycleUpgradeModeV1::ExternalProposal,
            LifecycleUpgradeModeV1::ExternalExecution,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
            LifecycleUpgradeModeV1::ObserveOnly,
        ],
        CanisterControlClassV1::UnknownUnsafe => vec![LifecycleUpgradeModeV1::Blocked],
    }
}

fn lifecycle_verification_requirements(
    verifier_required: bool,
) -> Vec<LifecycleVerificationRequirementV1> {
    let mut requirements = vec![
        LifecycleVerificationRequirementV1::LiveInventory,
        LifecycleVerificationRequirementV1::ControllerObservation,
        LifecycleVerificationRequirementV1::ModuleHash,
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
    ];
    if verifier_required {
        requirements.push(LifecycleVerificationRequirementV1::ProtectedCallReadiness);
    }
    requirements
}

const fn lifecycle_external_action_required(control_class: CanisterControlClassV1) -> bool {
    matches!(
        control_class,
        CanisterControlClassV1::CanicManagedPool
            | CanisterControlClassV1::ExternallyImported
            | CanisterControlClassV1::JointlyControlled
            | CanisterControlClassV1::UserControlled
    )
}

fn lifecycle_reason(control_class: CanisterControlClassV1) -> String {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => {
            "deployment authority can execute lifecycle directly".to_string()
        }
        CanisterControlClassV1::CanicManagedPool => {
            "Canic-managed pool lifecycle requires pool-aware external action".to_string()
        }
        CanisterControlClassV1::ExternallyImported => {
            "externally imported canister requires external controller action".to_string()
        }
        CanisterControlClassV1::JointlyControlled => {
            "jointly controlled canister requires non-deployment-controller consent".to_string()
        }
        CanisterControlClassV1::UserControlled => {
            "user-controlled canister requires user or delegated lifecycle action".to_string()
        }
        CanisterControlClassV1::UnknownUnsafe => {
            "unknown or unsafe controller state blocks lifecycle action".to_string()
        }
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

const fn required_external_action(lifecycle_mode: LifecycleModeV1) -> &'static str {
    match lifecycle_mode {
        LifecycleModeV1::DirectDeploymentAuthority => "none",
        LifecycleModeV1::ProposalRequired => "proposal_and_consent",
        LifecycleModeV1::DelegatedInstallRequired => "delegated_install_or_pool_policy",
        LifecycleModeV1::ExternalCompletionOnly => "external_controller_execution",
        LifecycleModeV1::VerifyOnly => "verify_external_completion",
        LifecycleModeV1::MustNotTouch | LifecycleModeV1::UnknownUnsafeBlocked => "blocked",
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

fn ensure_unique_lifecycle_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_authority_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), LifecycleAuthorityReportError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(LifecycleAuthorityReportError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_role_upgrade_subjects(
    rows: &[ExternalLifecycleRoleUpgradeV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_external_lifecycle_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePlanError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePlanError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_lifecycle_authority_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), LifecycleAuthorityReportError> {
    if value.trim().is_empty() {
        return Err(LifecycleAuthorityReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
