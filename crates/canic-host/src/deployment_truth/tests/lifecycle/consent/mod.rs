use super::super::*;

#[test]
fn external_upgrade_consent_evidence_packages_receipt_without_verification_claim() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();

    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");

    assert_eq!(evidence.evidence_id, "external-upgrade-consent-1");
    assert_eq!(evidence.proposal_id, proposal.proposal_id);
    assert_eq!(evidence.proposal_digest, proposal.proposal_digest);
    assert_eq!(evidence.receipt_id, receipt.receipt_id);
    assert_eq!(evidence.receipt_digest, receipt.receipt_digest);
    assert_eq!(evidence.consent_state, receipt.consent_state);
    assert_eq!(evidence.reported_by, receipt.reported_by);
    assert_eq!(evidence.consent_requirements, proposal.consent_requirements);
    assert_eq!(
        evidence.allowed_authorization_modes,
        proposal.allowed_authorization_modes
    );
    assert!(
        evidence
            .status_summary
            .contains("external controller execution")
    );
    assert_eq!(evidence.evidence_digest.len(), 64);
    validate_external_upgrade_consent_evidence(&evidence)
        .expect("consent evidence should validate");
    validate_external_upgrade_consent_evidence_for_receipt(&evidence, &proposal, &receipt)
        .expect("consent evidence should validate against source evidence");
    assert_json_round_trip(&evidence);
}

#[test]
fn external_upgrade_consent_evidence_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    let encoded = serde_json::to_value(&evidence).expect("evidence should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "evidence_digest",
            "proposal_id",
            "proposal_digest",
            "receipt_id",
            "receipt_digest",
            "subject",
            "canister_id",
            "role",
            "consent_state",
            "reported_by",
            "consent_requirements",
            "allowed_authorization_modes",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_consent_evidence_request_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let request = ExternalUpgradeConsentEvidenceRequest {
        evidence_id: "external-upgrade-consent-1".to_string(),
        proposal,
        receipt,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(&encoded, &["evidence_id", "proposal", "receipt"]);
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_consent_evidence_validation_rejects_stale_source() {
    let (mut proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    proposal.proposal_id = "other-proposal".to_string();

    let err =
        validate_external_upgrade_consent_evidence_for_receipt(&evidence, &proposal, &receipt)
            .expect_err("stale source should fail");

    std::assert_matches!(
        err,
        ExternalUpgradeConsentEvidenceError::Receipt(ExternalUpgradeReceiptError::SourceMismatch {
            field: "proposal_id"
        })
    );
}

#[test]
fn external_upgrade_consent_evidence_text_reports_passive_boundary() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");

    let text = external_upgrade_consent_evidence_text(&evidence);

    assert!(text.contains("External upgrade consent evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("consent_state: executed_externally"));
    assert!(text.contains("status_summary"));
}
