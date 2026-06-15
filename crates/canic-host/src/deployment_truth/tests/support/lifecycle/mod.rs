use super::*;

pub(in crate::deployment_truth::tests) fn sample_external_lifecycle_pending_artifacts()
-> (ExternalLifecyclePlanV1, ExternalLifecyclePendingReportV1) {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    (lifecycle_plan, pending_report)
}

pub(in crate::deployment_truth::tests) fn sample_external_upgrade_proposal_and_receipt()
-> (ExternalUpgradeProposalV1, ExternalUpgradeReceiptV1) {
    let (proposal, receipt, _) = sample_external_upgrade_proposal_receipt_and_check();
    (proposal, receipt)
}

pub(in crate::deployment_truth::tests) fn sample_external_upgrade_proposal_receipt_and_check() -> (
    ExternalUpgradeProposalV1,
    ExternalUpgradeReceiptV1,
    DeploymentCheckV1,
) {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );
    (proposal, receipt, check)
}
pub(in crate::deployment_truth::tests) fn assert_required_policy_requirement(
    policy: &ExternalUpgradeVerificationPolicyV1,
    requirement: LifecycleVerificationRequirementV1,
    expected_value: Option<&str>,
) {
    let row = policy
        .verification_requirements
        .iter()
        .find(|row| row.requirement == requirement)
        .expect("verification requirement should be present");
    assert_eq!(
        row.status,
        ExternalUpgradeVerificationRequirementStatusV1::Required
    );
    assert_eq!(row.expected_value.as_deref(), expected_value);
}

pub(in crate::deployment_truth::tests) fn matching_external_verification_observation(
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalUpgradeVerificationObservationV1 {
    ExternalUpgradeVerificationObservationV1 {
        source: ExternalVerificationObservationSourceV1::SuppliedObservation,
        deployment_check_id: None,
        deployment_check_digest: None,
        inventory_id: Some("inventory-verified".to_string()),
        observed_at: Some("2026-05-26T00:00:00Z".to_string()),
        live_inventory_observed: true,
        controller_observation_present: true,
        observed_control_class: Some(proposal.control_class),
        observed_module_hash: proposal.target_installed_module_hash.clone(),
        observed_canonical_embedded_config_sha256: proposal
            .target_canonical_embedded_config_sha256
            .clone(),
        protected_call_ready: Some(true),
    }
}

pub(in crate::deployment_truth::tests) fn sample_external_completion_sources() -> (
    ExternalUpgradeProposalV1,
    ExternalUpgradeConsentEvidenceV1,
    ExternalUpgradeVerificationCheckV1,
) {
    let (proposal, receipt, deployment_check) =
        sample_external_upgrade_proposal_receipt_and_check();
    let consent_evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-evidence-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("sample deployment check should produce verification observation");
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );
    (proposal, consent_evidence, verification_check)
}

pub(in crate::deployment_truth::tests) fn assert_inventory_verification_mismatch(
    mutate: impl FnOnce(&mut DeploymentCheckV1),
    requirement: LifecycleVerificationRequirementV1,
) {
    let (proposal, _, mut deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    mutate(&mut deployment_check);
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("deployment check should still produce verification observation");
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    assert_eq!(
        check.observation.source,
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    );
    assert_eq!(
        check.verification_result,
        ExternalUpgradeVerificationResultV1::Mismatch
    );
    assert!(check.requirement_results.iter().any(|row| {
        row.requirement == requirement
            && row.status == ExternalUpgradeVerificationRequirementStatusV1::Required
            && row.satisfied == Some(false)
    }));
    validate_external_upgrade_verification_check_for_deployment_check(
        &check,
        &policy,
        &deployment_check,
    )
    .expect("mismatch check should still validate against its source inventory");
}
