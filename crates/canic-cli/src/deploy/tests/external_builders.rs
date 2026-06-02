use super::super::external as deploy_external;
use super::DeployCommandError;
use super::fixtures::*;
use canic_host::deployment_truth::{
    CanisterControlClassV1, ConsentChannelKindV1, ExternalUpgradeCompletionReportRequest,
    ExternalUpgradeCompletionStatusV1, ExternalUpgradeConsentEvidenceRequest,
    ExternalUpgradeConsentStateV1, ExternalUpgradeVerificationCheckRequest,
    ExternalUpgradeVerificationObservationV1, ExternalUpgradeVerificationPolicyRequest,
    ExternalUpgradeVerificationReportRequest, ExternalUpgradeVerificationRequirementStatusV1,
    ExternalUpgradeVerificationResultV1, ExternalVerificationObservationSourceV1,
    LifecycleVerificationRequirementV1, ObservationStatusV1,
    external_upgrade_receipt_from_observation,
};

#[test]
fn external_lifecycle_plan_builder_uses_stable_local_ids() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let plan = deploy_external::build_lifecycle_plan(&check);

    assert_eq!(
        plan.lifecycle_plan_id,
        "local:local:demo:external-lifecycle-plan"
    );
    assert_eq!(
        plan.lifecycle_authority_report_id,
        "local:local:demo:lifecycle-authority-report"
    );
    assert_eq!(plan.deployment_plan_id, "plan-1");
    assert_eq!(plan.inventory_id, "inventory-1");
    assert_eq!(plan.proposed_external_role_upgrades.len(), 1);
    assert!(plan.directly_executable_role_upgrades.is_empty());
    assert_eq!(plan.lifecycle_plan_digest.len(), 64);
}

#[test]
fn external_lifecycle_check_builder_links_pending_report() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let pending_report = deploy_external::build_lifecycle_pending_report(&check);
    let lifecycle_check = deploy_external::build_lifecycle_check(&check);

    assert_eq!(
        lifecycle_check.check_id,
        "local:local:demo:external-lifecycle-check"
    );
    assert_eq!(lifecycle_check.pending_report_id, pending_report.report_id);
    assert_eq!(
        lifecycle_check.pending_report_digest,
        pending_report.report_digest
    );
    assert_eq!(lifecycle_check.pending_external_count, 1);
    assert_eq!(lifecycle_check.direct_upgrade_count, 0);
    assert_eq!(lifecycle_check.blocked_count, 0);
    assert_eq!(lifecycle_check.check_digest.len(), 64);
}

#[test]
fn external_lifecycle_handoff_builder_packages_pending_actions() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_check = deploy_external::build_lifecycle_check(&check);
    let handoff = deploy_external::build_lifecycle_handoff(&check);

    assert_eq!(
        handoff.handoff_id,
        "local:local:demo:external-lifecycle-handoff"
    );
    assert_eq!(handoff.lifecycle_check_id, lifecycle_check.check_id);
    assert_eq!(handoff.lifecycle_check_digest, lifecycle_check.check_digest);
    assert_eq!(handoff.handoff_actions.len(), 1);
    assert_eq!(
        handoff.handoff_actions[0].consent_channel_kind,
        ConsentChannelKindV1::GeneratedCommand
    );
    assert_eq!(handoff.handoff_digest.len(), 64);
}

#[test]
fn external_proposal_report_builder_delegates_to_lifecycle_plan() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_plan = deploy_external::build_lifecycle_plan(&check);
    let report = deploy_external::build_upgrade_proposal_report(&check);

    assert_eq!(
        report.report_id,
        "local:local:demo:external-upgrade-proposals"
    );
    assert_eq!(report.report_digest.len(), 64);
    assert_eq!(report.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
    assert_eq!(
        report.lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(report.deployment_plan_id, "plan-1");
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.proposals.len(), 1);
    assert_eq!(
        report.proposals[0].lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(
        report.proposals[0].required_external_action,
        "external_controller_execution"
    );
}

#[test]
fn external_pending_report_builder_links_plan_and_proposals() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_plan = deploy_external::build_lifecycle_plan(&check);
    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let pending_report = deploy_external::build_lifecycle_pending_report(&check);

    assert_eq!(
        pending_report.report_id,
        "local:local:demo:external-lifecycle-pending"
    );
    assert_eq!(
        pending_report.lifecycle_plan_id,
        lifecycle_plan.lifecycle_plan_id
    );
    assert_eq!(
        pending_report.lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(pending_report.proposal_report_id, proposal_report.report_id);
    assert_eq!(
        pending_report.proposal_report_digest,
        proposal_report.report_digest
    );
    assert_eq!(pending_report.pending_external_count, 1);
    assert_eq!(pending_report.direct_upgrade_count, 0);
    assert_eq!(pending_report.blocked_count, 0);
    assert_eq!(
        pending_report.pending_external_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert_eq!(pending_report.report_digest.len(), 64);
}

#[test]
fn external_critical_fix_report_builder_links_pending_report() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let pending_report = deploy_external::build_lifecycle_pending_report(&check);
    let critical_fix =
        deploy_external::build_critical_fix_report(&check, "fix-2026-05", "critical");

    assert_eq!(
        critical_fix.report_id,
        "local:local:demo:critical-external-fix"
    );
    assert_eq!(critical_fix.fix_id, "fix-2026-05");
    assert_eq!(critical_fix.severity, "critical");
    assert_eq!(critical_fix.pending_report_id, pending_report.report_id);
    assert_eq!(
        critical_fix.pending_report_digest,
        pending_report.report_digest
    );
    assert_eq!(critical_fix.externally_blocked_roles, vec!["root"]);
    assert_eq!(critical_fix.required_external_actions.len(), 1);
    assert!(!critical_fix.residual_exposure.is_empty());
    assert_eq!(critical_fix.report_digest.len(), 64);
}

#[test]
fn external_consent_evidence_builder_delegates_to_receipt_validation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let evidence =
        deploy_external::build_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
            evidence_id: "external-upgrade-consent-1".to_string(),
            proposal,
            receipt,
        })
        .expect("consent evidence should build");

    assert_eq!(evidence.evidence_id, "external-upgrade-consent-1");
    assert_eq!(evidence.receipt_id, "external-upgrade-receipt-1");
    assert!(!evidence.status_summary.is_empty());
    assert_eq!(evidence.evidence_digest.len(), 64);
}

#[test]
fn external_verification_policy_builder_uses_proposal_requirements() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal,
        },
    );

    assert_eq!(policy.policy_id, "external-upgrade-verification-policy-1");
    assert_eq!(policy.verification_requirements.len(), 5);
    assert!(policy.verification_requirements.iter().any(|row| {
        row.requirement == LifecycleVerificationRequirementV1::LiveInventory
            && row.status == ExternalUpgradeVerificationRequirementStatusV1::Required
    }));
    assert!(!policy.status_summary.is_empty());
    assert_eq!(policy.policy_digest.len(), 64);
}

#[test]
fn external_verification_check_builder_evaluates_supplied_observation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: Some(ExternalUpgradeVerificationObservationV1 {
                source: ExternalVerificationObservationSourceV1::SuppliedObservation,
                deployment_check_id: None,
                deployment_check_digest: None,
                inventory_id: Some("inventory-verified".to_string()),
                observed_at: Some("2026-05-26T00:00:00Z".to_string()),
                live_inventory_observed: true,
                controller_observation_present: true,
                observed_control_class: Some(proposal.control_class),
                observed_module_hash: proposal.target_installed_module_hash,
                observed_canonical_embedded_config_sha256: proposal
                    .target_canonical_embedded_config_sha256,
                protected_call_ready: Some(true),
            }),
            deployment_check: None,
        },
    )
    .expect("verification check should build");

    assert_eq!(
        verification_check.check_id,
        "external-upgrade-verification-check-1"
    );
    assert_eq!(
        verification_check.verification_result,
        ExternalUpgradeVerificationResultV1::Pending
    );
    assert_eq!(verification_check.check_digest.len(), 64);
}

#[test]
fn external_verification_check_builder_verifies_deployment_truth_inventory() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    check.inventory.observed_canisters[0].module_hash = Some("module".to_string());
    check.inventory.observed_canisters[0].canonical_embedded_config_digest =
        Some(sample_sha256("c"));
    check.inventory.observed_verifier_readiness.status = ObservationStatusV1::Observed;

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal,
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: None,
            deployment_check: Some(check.clone()),
        },
    )
    .expect("inventory-backed verification check should build");

    assert_eq!(
        verification_check.observation.source,
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    );
    assert_eq!(
        verification_check
            .observation
            .deployment_check_id
            .as_deref(),
        Some(check.check_id.as_str())
    );
    assert!(
        verification_check.requirement_results.iter().all(|row| {
            row.status == ExternalUpgradeVerificationRequirementStatusV1::NotRequired
                || row.satisfied == Some(true)
        }),
        "{:?}",
        verification_check.requirement_results
    );
    assert_eq!(
        verification_check.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
    );
}

#[test]
fn external_verification_check_builder_rejects_ambiguous_observation_sources() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let observation = ExternalUpgradeVerificationObservationV1 {
        source: ExternalVerificationObservationSourceV1::SuppliedObservation,
        deployment_check_id: None,
        deployment_check_digest: None,
        inventory_id: Some("inventory-verified".to_string()),
        observed_at: Some("2026-05-26T00:00:00Z".to_string()),
        live_inventory_observed: true,
        controller_observation_present: true,
        observed_control_class: Some(proposal.control_class),
        observed_module_hash: proposal.target_installed_module_hash,
        observed_canonical_embedded_config_sha256: proposal.target_canonical_embedded_config_sha256,
        protected_call_ready: Some(true),
    };

    let both_err = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy: policy.clone(),
            observation: Some(observation),
            deployment_check: Some(check.clone()),
        },
    )
    .expect_err("both observation sources should be rejected");
    std::assert_matches!(both_err, DeployCommandError::Blocked(_));

    let neither_err = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: None,
            deployment_check: None,
        },
    )
    .expect_err("missing observation source should be rejected");
    std::assert_matches!(neither_err, DeployCommandError::Blocked(_));
}

#[test]
fn external_completion_report_builder_links_consent_and_verification() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let consent_evidence =
        deploy_external::build_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
            evidence_id: "external-upgrade-consent-1".to_string(),
            proposal: proposal.clone(),
            receipt,
        })
        .expect("consent evidence should build");
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: Some(ExternalUpgradeVerificationObservationV1 {
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
            }),
            deployment_check: None,
        },
    )
    .expect("verification check should build");

    let completion =
        deploy_external::build_upgrade_completion_report(ExternalUpgradeCompletionReportRequest {
            report_id: "external-upgrade-completion-1".to_string(),
            proposal,
            consent_evidence,
            verification_check,
        })
        .expect("completion report should build");

    assert_eq!(completion.report_id, "external-upgrade-completion-1");
    assert_eq!(
        completion.completion_status,
        ExternalUpgradeCompletionStatusV1::AwaitingConsent
    );
    assert_eq!(completion.report_digest.len(), 64);
}

#[test]
fn external_verification_report_builder_delegates_to_receipt_validation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let report = deploy_external::build_upgrade_verification_report(
        ExternalUpgradeVerificationReportRequest {
            report_id: "external-upgrade-verification-1".to_string(),
            proposal,
            receipt,
        },
    )
    .expect("verification report should build");

    assert_eq!(report.report_id, "external-upgrade-verification-1");
    assert_eq!(report.receipt_id, "external-upgrade-receipt-1");
    assert!(!report.status_summary.is_empty());
    assert_eq!(report.report_digest.len(), 64);
}
