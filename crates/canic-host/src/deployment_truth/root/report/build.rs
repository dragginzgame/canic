use super::super::super::*;
use super::{
    super::digest::deployment_root_verification_report_digest,
    checks::{
        root_verification_blockers, root_verification_evidence_checks,
        root_verification_identity_checks,
    },
    shared::{root_verification_next_actions, root_verification_transition},
};

/// Build a passive root-verification report from an existing
/// deployment-truth check.
///
/// This report can prove evidence consistency, but it does not mutate local
/// deployment state or record verified root state.
#[must_use]
pub fn deployment_root_verification_report_from_check(
    request: DeploymentRootVerificationRequestV1,
) -> DeploymentRootVerificationReportV1 {
    let check = &request.deployment_check;
    let observed_root = check.inventory.observed_root.as_ref();
    let identity_checks = root_verification_identity_checks(&request, check, observed_root);
    let evidence_checks = root_verification_evidence_checks(&request, check, observed_root);
    let blockers = root_verification_blockers(&identity_checks, &evidence_checks, check);

    let evidence_status = if blockers.is_empty() {
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
    } else {
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    };
    let state_transition =
        root_verification_transition(evidence_status, request.current_root_verification);
    let recommended_next_actions = root_verification_next_actions(evidence_status);
    let mut report = DeploymentRootVerificationReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        report_digest: String::new(),
        requested_at: request.requested_at,
        evidence_status,
        state_transition,
        deployment_name: request.deployment_name,
        environment: request.environment,
        expected_fleet_template: request.expected_fleet_template,
        expected_root_principal: request.expected_root_principal,
        observed_deployment_name: observed_root.map(|root| root.deployment_name.clone()),
        observed_environment: observed_root.map(|root| root.environment.clone()),
        observed_fleet_template: observed_root.map(|root| root.fleet_template.clone()),
        observed_root_principal: observed_root.map(|root| root.root_principal.clone()),
        observed_root_canister_id: observed_root.map(|root| root.observed_canister_id.clone()),
        observed_root_observation_source: observed_root.map(|root| root.observation_source),
        source: request.source,
        source_check_id: check.check_id.clone(),
        source_check_digest: stable_json_sha256_hex(check),
        source_deployment_plan_id: check.plan.plan_id.clone(),
        source_deployment_plan_digest: stable_json_sha256_hex(&check.plan),
        source_inventory_id: check.inventory.inventory_id.clone(),
        source_inventory_digest: stable_json_sha256_hex(&check.inventory),
        current_root_verification: request.current_root_verification,
        identity_checks,
        evidence_checks,
        blockers,
        warnings: check.report.warnings.clone(),
        recommended_next_actions,
    };
    report.report_digest = deployment_root_verification_report_digest(&report);
    report
}
