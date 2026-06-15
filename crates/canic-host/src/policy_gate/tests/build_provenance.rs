use super::*;

#[test]
fn build_provenance_policy_passes_matching_payload() {
    let report = evaluate_policy_for_test(BUILD_PROVENANCE_POLICY, sample_envelope());

    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    assert!(report.findings.is_empty());
    assert!(
        report.requirements.iter().any(
            |requirement| requirement.requirement_id == "build_provenance.require_clean_source"
        )
    );
}

#[test]
fn build_provenance_policy_rejects_dirty_or_unknown_source() {
    let mut dirty = sample_build_provenance_payload();
    dirty.source.dirty = Some(true);
    dirty.source.dirty_policy = SourceDirtyPolicyV1::DirtyRecorded;
    let dirty_report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(dirty).expect("payload json")),
    );

    assert_eq!(dirty_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    assert!(
        dirty_report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.source_not_clean")
    );

    let mut unknown = sample_build_provenance_payload();
    unknown.source.vcs = SourceVcsV1::Unknown;
    unknown.source.dirty = None;
    unknown.source.dirty_policy = SourceDirtyPolicyV1::Unknown;
    let unknown_report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(unknown).expect("payload json")),
    );

    assert_eq!(unknown_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
}

#[test]
fn build_provenance_policy_requires_cargo_lock_and_gzip_wasm() {
    let mut no_lock = sample_build_provenance_payload();
    no_lock.cargo.cargo_lock_sha256 = None;
    let no_lock_report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(no_lock).expect("payload json")),
    );

    assert_eq!(
        no_lock_report.gate_exit_class,
        ExitClassV1::MissingRequiredEvidence
    );
    assert!(
        no_lock_report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.cargo_lock_missing")
    );

    let mut no_gzip = sample_build_provenance_payload();
    no_gzip
        .artifacts
        .retain(|artifact| artifact.artifact_kind != ArtifactProvenanceKindV1::WasmGzip);
    let no_gzip_report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(no_gzip).expect("payload json")),
    );

    assert_eq!(
        no_gzip_report.gate_exit_class,
        ExitClassV1::MissingRequiredEvidence
    );
    assert!(
        no_gzip_report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.wasm_gzip_missing")
    );
}

#[test]
fn build_provenance_policy_requires_sha256_artifact_evidence() {
    let mut payload = sample_build_provenance_payload();
    payload.artifacts[0].sha256 = "not-a-sha".to_string();
    let report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(payload).expect("payload json")),
    );

    assert_eq!(report.gate_exit_class, ExitClassV1::MissingRequiredEvidence);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.sha256_missing_or_invalid")
    );
}

#[test]
fn build_provenance_policy_requires_package_identity_to_match_target() {
    let mut payload = sample_build_provenance_payload();
    payload.cargo.package_metadata_role = "other".to_string();
    let report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(serde_json::to_value(payload).expect("payload json")),
    );

    assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.package_identity_mismatch")
    );
}

#[test]
fn build_provenance_policy_rejects_wrong_or_invalid_payload() {
    let mut wrong_schema = sample_envelope();
    wrong_schema.payload_schema = PayloadSchemaRefV1::stable("canic.adoption_report.v1", "1");
    let wrong_schema_report = evaluate_policy_for_test(BUILD_PROVENANCE_POLICY, wrong_schema);

    assert_eq!(
        wrong_schema_report.gate_exit_class,
        ExitClassV1::BlockedByPolicy
    );
    assert!(
        wrong_schema_report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.payload_schema")
    );

    let invalid_report = evaluate_policy_for_test(
        BUILD_PROVENANCE_POLICY,
        sample_envelope_with_payload(json!({ "schema_version": 1 })),
    );

    assert_eq!(invalid_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    assert!(
        invalid_report
            .findings
            .iter()
            .any(|finding| finding.code == "policy.build_provenance.invalid_payload")
    );
}
