//! Module: canic_cli::deploy::plan::outcome
//!
//! Responsibility: derive final deployment-plan status, comparison, actions, and previews.
//! Does not own: evidence construction, diagnostic classification, report assembly, or rendering.
//! Boundary: maps the resolved plan and diagnostics into deterministic outcome fields.

use crate::deploy::plan::{
    command::DeployPlanOptions,
    diagnostics::is_observed_state_drift_assumption,
    evidence::verifier_readiness_required,
    report::{
        CATEGORY_ARTIFACT, ComparisonStatus, FUTURE_APPLY_PREVIEW_PHASE, OP_APPLY_POLICY,
        OP_CREATE_CANISTER, OP_INSTALL_WASM, OP_REGISTER_CHILD, OP_REGISTER_ROOT,
        OP_SET_CONTROLLERS, OP_UPGRADE_WASM, OP_UPLOAD_ARTIFACT, OP_VERIFY_READINESS,
        OP_VERIFY_TOPOLOGY, PROPOSED_OPERATION_NOT_EXECUTED, PlanDiagnostic,
        PlanDiagnosticSeverity, PlanStatus, ProposedOperationKind, ProposedOperationLabel,
        SEVERITY_UNSUPPORTED,
    },
};

use canic_host::deployment_truth::{DeploymentAssumptionKindV1, DeploymentPlanV1};

pub(super) fn proposed_operations(plan: &DeploymentPlanV1) -> Vec<ProposedOperationLabel> {
    let mut operations = Vec::new();
    for canister in &plan.expected_canisters {
        if canister.canister_id.is_none() {
            operations.push(operation(OP_CREATE_CANISTER, &canister.role));
        }
    }
    for canister in &plan.expected_pool {
        if canister.canister_id.is_none() {
            let subject = pool_operation_subject(&canister.pool, canister.role.as_deref());
            operations.push(operation(OP_CREATE_CANISTER, &subject));
        }
    }
    for canister in &plan.expected_canisters {
        if canister.canister_id.is_none() {
            operations.push(operation(
                registration_operation_label(&canister.role),
                &canister.role,
            ));
        }
    }
    for canister in &plan.expected_pool {
        if canister.canister_id.is_none() {
            let subject = pool_operation_subject(&canister.pool, canister.role.as_deref());
            operations.push(operation(OP_REGISTER_CHILD, &subject));
        }
    }
    for artifact in &plan.role_artifacts {
        operations.push(operation(OP_UPLOAD_ARTIFACT, &artifact.role));
        operations.push(operation(
            wasm_operation_label(plan, &artifact.role),
            &artifact.role,
        ));
    }
    if !plan.authority_profile.expected_controllers.is_empty() {
        operations.push(operation(
            OP_APPLY_POLICY,
            &plan.deployment_identity.deployment_name,
        ));
        operations.push(operation(
            OP_SET_CONTROLLERS,
            &plan.deployment_identity.deployment_name,
        ));
    }
    if verifier_readiness_required(plan) {
        operations.push(operation(
            OP_VERIFY_READINESS,
            &plan.deployment_identity.deployment_name,
        ));
    }
    operations.push(operation(
        OP_VERIFY_TOPOLOGY,
        &plan.deployment_identity.deployment_name,
    ));
    sort_proposed_operations(&mut operations);
    operations
}

fn registration_operation_label(role: &str) -> ProposedOperationKind {
    if role == "root" {
        OP_REGISTER_ROOT
    } else {
        OP_REGISTER_CHILD
    }
}

fn pool_operation_subject(pool: &str, role: Option<&str>) -> String {
    match role {
        Some(role) => format!("{pool}:{role}"),
        None => pool.to_string(),
    }
}

fn wasm_operation_label(plan: &DeploymentPlanV1, role: &str) -> ProposedOperationKind {
    if plan
        .expected_canisters
        .iter()
        .any(|canister| canister.role == role && canister.canister_id.is_some())
    {
        OP_UPGRADE_WASM
    } else {
        OP_INSTALL_WASM
    }
}

pub(super) fn operation(label: ProposedOperationKind, subject: &str) -> ProposedOperationLabel {
    ProposedOperationLabel {
        phase: FUTURE_APPLY_PREVIEW_PHASE,
        label,
        subject: subject.to_string(),
        status: PROPOSED_OPERATION_NOT_EXECUTED,
    }
}

pub(super) fn next_actions(
    options: &DeployPlanOptions,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> Vec<String> {
    let mut actions = Vec::new();
    if !blockers.is_empty() {
        actions.push("fix blocker diagnostics before designing apply".to_string());
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        actions.push("resolve warnings before designing apply".to_string());
    }
    if has_artifact_diagnostics(blockers)
        || has_artifact_diagnostics(warnings)
        || has_artifact_diagnostics(assumptions)
    {
        actions
            .push("run canic build or provide a build profile with resolved artifacts".to_string());
    }
    actions.push(format!(
        "run canic medic deployment {} if operator readiness is uncertain",
        options.deployment
    ));
    actions
}

fn has_artifact_diagnostics(diagnostics: &[PlanDiagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.category == CATEGORY_ARTIFACT)
}

pub(super) fn aggregate_status(
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> PlanStatus {
    if blockers
        .iter()
        .any(|diagnostic| diagnostic.severity == SEVERITY_UNSUPPORTED)
    {
        return PlanStatus::Unsupported;
    }
    if !blockers.is_empty() {
        return PlanStatus::Blocked;
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        return PlanStatus::Warning;
    }
    PlanStatus::Planned
}

pub(super) fn comparison_status(
    plan: &DeploymentPlanV1,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> ComparisonStatus {
    if !blockers.is_empty() {
        return ComparisonStatus::NotRequested;
    }

    if has_observed_state_drift(plan) {
        return ComparisonStatus::ComparedWithDrift;
    }

    if has_missing_observed_state(plan) {
        return ComparisonStatus::NotAvailable;
    }

    if plan.trust_domain.root_trust_anchor.is_none() {
        return ComparisonStatus::NotRequested;
    }

    if !warnings.is_empty() || !assumptions.is_empty() {
        ComparisonStatus::ComparedWithWarnings
    } else {
        ComparisonStatus::Compared
    }
}

fn has_observed_state_drift(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions
        .iter()
        .any(is_observed_state_drift_assumption)
}

fn has_missing_observed_state(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.has_kind(DeploymentAssumptionKindV1::LocalStateMissing)
            || assumption.has_kind(DeploymentAssumptionKindV1::LocalStateReadFailed)
    })
}

pub(super) fn sort_diagnostics(diagnostics: &mut [PlanDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        diagnostic_severity_rank(left.severity)
            .cmp(&diagnostic_severity_rank(right.severity))
            .then_with(|| left.severity.label().cmp(right.severity.label()))
            .then_with(|| left.category.label().cmp(right.category.label()))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.source.label().cmp(right.source.label()))
    });
}

const fn diagnostic_severity_rank(severity: PlanDiagnosticSeverity) -> u8 {
    severity.sort_rank()
}

pub(super) fn sort_proposed_operations(operations: &mut Vec<ProposedOperationLabel>) {
    operations.sort_by(|left, right| {
        left.phase
            .cmp(&right.phase)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.status.cmp(&right.status))
    });
    operations.dedup();
}
