use super::super::*;

#[test]
fn external_upgrade_completion_report_marks_verified_completion() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(report.report_id, "external-upgrade-completion-1");
    assert_eq!(report.proposal_id, proposal.proposal_id);
    assert_eq!(report.consent_evidence_id, consent_evidence.evidence_id);
    assert_eq!(report.verification_check_id, verification_check.check_id);
    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerifiedComplete
    );
    assert!(report.blockers.is_empty());
    assert_eq!(report.report_digest.len(), 64);
    validate_external_upgrade_completion_report(&report).expect("report should validate");
    validate_external_upgrade_completion_report_for_evidence(
        &report,
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("report should validate against evidence");
    assert_json_round_trip(&report);
}

#[test]
fn external_upgrade_completion_report_does_not_complete_from_supplied_observation() {
    let (proposal, consent_evidence, _) = sample_external_completion_sources();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(
        report.verification_observation_source,
        ExternalVerificationObservationSourceV1::SuppliedObservation
    );
    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::AwaitingVerification
    );
    assert_ne!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerifiedComplete
    );
    assert!(!report.blockers.is_empty());
}

#[test]
fn external_upgrade_completion_report_marks_verification_failed() {
    let (proposal, consent_evidence, _) = sample_external_completion_sources();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let mut observation = matching_external_verification_observation(&proposal);
    observation.observed_module_hash = Some("wrong-module-hash".to_string());
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerificationFailed
    );
    assert_eq!(report.blockers.len(), 1);
}

#[test]
fn external_upgrade_completion_report_validation_rejects_stale_evidence() {
    let (proposal, mut consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");
    consent_evidence.evidence_id = "other-consent-evidence".to_string();

    let err = validate_external_upgrade_completion_report_for_evidence(
        &report,
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect_err("stale evidence should fail");

    assert_eq!(
        err,
        ExternalUpgradeCompletionReportError::ConsentEvidence(
            ExternalUpgradeConsentEvidenceError::DigestMismatch {
                field: "evidence_digest"
            }
        )
    );
}

#[test]
fn external_upgrade_completion_report_json_shape_is_stable() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");
    let encoded = serde_json::to_value(&report).expect("report should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "report_digest",
            "proposal_id",
            "proposal_digest",
            "consent_evidence_id",
            "consent_evidence_digest",
            "verification_check_id",
            "verification_check_digest",
            "subject",
            "canister_id",
            "role",
            "consent_state",
            "verification_result",
            "verification_observation_source",
            "completion_status",
            "blockers",
            "next_actions",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_completion_report_request_json_shape_is_stable() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let request = ExternalUpgradeCompletionReportRequest {
        report_id: "external-upgrade-completion-1".to_string(),
        proposal,
        consent_evidence,
        verification_check,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(
        &encoded,
        &[
            "report_id",
            "proposal",
            "consent_evidence",
            "verification_check",
        ],
    );
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_completion_report_text_reports_passive_boundary() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    let text = external_upgrade_completion_report_text(&report);

    assert!(text.contains("External upgrade completion report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("live_lookup: none"));
    assert!(text.contains("completion_status: verified_complete"));
}
