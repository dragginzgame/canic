use super::super::DeployCommandError;
use canic_host::deployment_truth::{
    CriticalExternalFixReportV1, DeploymentCheckV1, ExternalLifecycleCheckV1,
    ExternalLifecycleHandoffV1, ExternalLifecyclePendingReportV1, ExternalLifecyclePlanV1,
    ExternalUpgradeCompletionReportRequest, ExternalUpgradeCompletionReportV1,
    ExternalUpgradeConsentEvidenceRequest, ExternalUpgradeConsentEvidenceV1,
    ExternalUpgradeProposalReportV1, ExternalUpgradeVerificationCheckRequest,
    ExternalUpgradeVerificationCheckV1, ExternalUpgradeVerificationPolicyRequest,
    ExternalUpgradeVerificationPolicyV1, ExternalUpgradeVerificationReportRequest,
    ExternalUpgradeVerificationReportV1, critical_external_fix_report_from_pending,
    external_lifecycle_check_from_reports, external_lifecycle_handoff_from_reports,
    external_lifecycle_pending_report_from_plan, external_lifecycle_plan_from_check,
    external_upgrade_completion_report_from_evidence,
    external_upgrade_consent_evidence_from_receipt,
    external_upgrade_proposal_report_from_lifecycle_plan,
    external_upgrade_verification_check_from_policy,
    external_upgrade_verification_observation_from_check,
    external_upgrade_verification_policy_from_proposal,
    external_upgrade_verification_report_from_receipt,
    validate_external_upgrade_verification_check_for_deployment_check,
    validate_external_upgrade_verification_check_for_policy,
};

pub fn build_lifecycle_plan(check: &DeploymentCheckV1) -> ExternalLifecyclePlanV1 {
    external_lifecycle_plan_from_check(
        local_lifecycle_plan_id(check),
        local_lifecycle_authority_report_id(check),
        check,
    )
}

pub fn build_upgrade_proposal_report(check: &DeploymentCheckV1) -> ExternalUpgradeProposalReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    )
}

pub fn build_lifecycle_pending_report(
    check: &DeploymentCheckV1,
) -> ExternalLifecyclePendingReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    )
}

pub fn build_lifecycle_check(check: &DeploymentCheckV1) -> ExternalLifecycleCheckV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    external_lifecycle_check_from_reports(
        local_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
}

pub fn build_lifecycle_handoff(check: &DeploymentCheckV1) -> ExternalLifecycleHandoffV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    let lifecycle_check = external_lifecycle_check_from_reports(
        local_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    external_lifecycle_handoff_from_reports(
        local_handoff_id(check),
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
}

pub fn build_critical_fix_report(
    check: &DeploymentCheckV1,
    fix_id: &str,
    severity: &str,
) -> CriticalExternalFixReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    critical_external_fix_report_from_pending(
        local_critical_fix_report_id(check),
        fix_id,
        severity,
        &lifecycle_plan,
        &pending_report,
    )
}

pub fn build_upgrade_consent_evidence(
    request: ExternalUpgradeConsentEvidenceRequest,
) -> Result<ExternalUpgradeConsentEvidenceV1, DeployCommandError> {
    external_upgrade_consent_evidence_from_receipt(
        request.evidence_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub fn build_upgrade_verification_policy(
    request: ExternalUpgradeVerificationPolicyRequest,
) -> ExternalUpgradeVerificationPolicyV1 {
    external_upgrade_verification_policy_from_proposal(request.policy_id, &request.proposal)
}

pub fn build_upgrade_verification_check(
    request: ExternalUpgradeVerificationCheckRequest,
) -> Result<ExternalUpgradeVerificationCheckV1, DeployCommandError> {
    let observation = match (request.observation, request.deployment_check) {
        (Some(observation), None) => observation,
        (None, Some(deployment_check)) => {
            let observation = external_upgrade_verification_observation_from_check(
                &request.policy,
                &deployment_check,
            )
            .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
            let check = external_upgrade_verification_check_from_policy(
                request.check_id,
                &request.policy,
                observation,
            );
            validate_external_upgrade_verification_check_for_deployment_check(
                &check,
                &request.policy,
                &deployment_check,
            )
            .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
            return Ok(check);
        }
        (Some(_), Some(_)) => {
            return Err(DeployCommandError::Blocked(
                "external verification check request must provide either observation or deployment_check, not both"
                    .to_string(),
            ));
        }
        (None, None) => {
            return Err(DeployCommandError::Blocked(
                "external verification check request must provide observation or deployment_check"
                    .to_string(),
            ));
        }
    };
    let check = external_upgrade_verification_check_from_policy(
        request.check_id,
        &request.policy,
        observation,
    );
    validate_external_upgrade_verification_check_for_policy(&check, &request.policy)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(check)
}

pub fn build_upgrade_completion_report(
    request: ExternalUpgradeCompletionReportRequest,
) -> Result<ExternalUpgradeCompletionReportV1, DeployCommandError> {
    external_upgrade_completion_report_from_evidence(
        request.report_id,
        &request.proposal,
        &request.consent_evidence,
        &request.verification_check,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub fn build_upgrade_verification_report(
    request: ExternalUpgradeVerificationReportRequest,
) -> Result<ExternalUpgradeVerificationReportV1, DeployCommandError> {
    external_upgrade_verification_report_from_receipt(
        request.report_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn local_lifecycle_plan_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-plan")
}

fn local_check_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-check")
}

fn local_handoff_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-handoff")
}

fn local_lifecycle_authority_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "lifecycle-authority-report")
}

fn local_proposal_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-upgrade-proposals")
}

fn local_pending_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-pending")
}

fn local_critical_fix_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "critical-external-fix")
}

fn local_artifact_id(check: &DeploymentCheckV1, suffix: &str) -> String {
    format!(
        "local:{}:{}:{suffix}",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    )
}
