use super::*;

pub(in crate::deployment_truth::tests) fn sample_root_verification_check() -> DeploymentCheckV1 {
    sample_check(
        sample_root_verification_plan(),
        sample_root_verification_inventory(),
    )
}

pub(in crate::deployment_truth::tests) fn sample_root_verification_receipt()
-> DeploymentRootVerificationReceiptV1 {
    let report = deployment_root_verification_report_from_check(sample_root_verification_request(
        sample_root_verification_check(),
    ));
    let mut receipt = DeploymentRootVerificationReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: "receipt-root-verification".to_string(),
        receipt_digest: String::new(),
        deployment_name: report.deployment_name,
        network: report.network,
        fleet_template: report.expected_fleet_template,
        root_principal: report.expected_root_principal,
        previous_root_verification: DeploymentRootVerificationStateV1::NotVerified,
        new_root_verification: DeploymentRootVerificationStateV1::Verified,
        state_transition:
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified,
        source_report_id: report.report_id,
        source_report_digest: report.report_digest,
        source_report_requested_at: report.requested_at,
        source_report_source: report.source,
        source_report_evidence_status: report.evidence_status,
        source_report_current_root_verification: report.current_root_verification,
        source_report_state_transition: report.state_transition,
        source_root_observation_source: report
            .observed_root_observation_source
            .expect("observed source"),
        source_observed_root_canister_id: report
            .observed_root_canister_id
            .expect("observed root canister id"),
        source_check_id: report.source_check_id,
        source_check_digest: report.source_check_digest,
        source_deployment_plan_id: report.source_deployment_plan_id,
        source_deployment_plan_digest: report.source_deployment_plan_digest,
        source_inventory_id: report.source_inventory_id,
        source_inventory_digest: report.source_inventory_digest,
        verified_at_unix_secs: 100,
        local_state_path: ".canic/local/deployments/demo.json".to_string(),
        local_state_digest_before: "a".repeat(64),
        local_state_digest_after: "b".repeat(64),
        warnings: Vec::new(),
    };
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);
    receipt
}

pub(in crate::deployment_truth::tests) fn sample_root_verification_plan() -> DeploymentPlanV1 {
    let mut plan = sample_plan();
    plan.deployment_identity.deployment_name = "demo".to_string();
    plan
}

pub(in crate::deployment_truth::tests) fn sample_root_verification_inventory()
-> DeploymentInventoryV1 {
    let mut inventory = sample_matching_inventory();
    if let Some(identity) = inventory.observed_identity.as_mut() {
        identity.deployment_name = "demo".to_string();
    }
    inventory
}

pub(in crate::deployment_truth::tests) fn sample_root_verification_request(
    deployment_check: DeploymentCheckV1,
) -> DeploymentRootVerificationRequestV1 {
    DeploymentRootVerificationRequestV1 {
        report_id: "root-verification-report-1".to_string(),
        requested_at: "2026-05-27T00:00:00Z".to_string(),
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        expected_fleet_template: "root".to_string(),
        expected_root_principal: "aaaaa-aa".to_string(),
        current_root_verification: DeploymentRootVerificationStateV1::NotVerified,
        source: DeploymentRootVerificationSourceV1::DeploymentTruthCheck,
        deployment_check,
    }
}
