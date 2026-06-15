use super::*;

#[test]
fn project_evidence_manifest_gate_evaluates_required_envelope() {
    let root = temp_dir("canic-policy-manifest-pass");
    fs::create_dir_all(&root).expect("create root");
    let policy_path = root.join("policy.toml");
    let manifest_path = root.join("evidence.toml");
    let envelope_path = root.join("build.json");
    fs::write(&policy_path, BUILD_PROVENANCE_POLICY).expect("write policy");
    fs::write(
        &envelope_path,
        serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
    )
    .expect("write envelope");
    let manifest_source = sample_manifest_source("build.json", true);
    fs::write(&manifest_path, &manifest_source).expect("write manifest");

    let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
        policy_source: BUILD_PROVENANCE_POLICY,
        policy_path: &policy_path,
        manifest_source: &manifest_source,
        manifest_path: &manifest_path,
        fingerprint_root: &root,
    })
    .expect("evaluate manifest gate");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    assert_eq!(report.evidence.len(), 1);
    assert_eq!(report.evidence[0].status, PolicyEvaluationStatusV1::Passed);
    assert!(report.evidence[0].policy_report.is_some());
}

#[test]
fn project_evidence_manifest_gate_reports_missing_required_and_optional_evidence() {
    let required_report = evaluate_manifest_gate_for_test(
        &sample_manifest_source("missing.json", true),
        BUILD_PROVENANCE_POLICY,
    );

    assert_eq!(
        required_report.gate_exit_class,
        ExitClassV1::MissingRequiredEvidence
    );
    assert_eq!(
        required_report.evidence[0].status,
        PolicyEvaluationStatusV1::Failed
    );
    assert_eq!(
        required_report.evidence[0].findings[0].code,
        "policy.manifest.required_evidence_missing"
    );

    let optional_report = evaluate_manifest_gate_for_test(
        &sample_manifest_source("missing.json", false),
        BUILD_PROVENANCE_POLICY,
    );

    assert_eq!(
        optional_report.gate_exit_class,
        ExitClassV1::SuccessWithWarnings
    );
    assert_eq!(
        optional_report.evidence[0].status,
        PolicyEvaluationStatusV1::Passed
    );
    assert_eq!(
        optional_report.evidence[0].findings[0].code,
        "policy.manifest.optional_evidence_missing"
    );
}

#[test]
fn project_evidence_manifest_gate_checks_target_and_payload_schema_expectations() {
    let mut wrong_schema = sample_envelope();
    wrong_schema.payload_schema = PayloadSchemaRefV1::stable("canic.other.v1", "1");
    let wrong_schema_report = evaluate_manifest_gate_with_envelope(
        &sample_manifest_source("build.json", true),
        wrong_schema,
    );

    assert_eq!(
        wrong_schema_report.gate_exit_class,
        ExitClassV1::BlockedByPolicy
    );
    assert!(
        wrong_schema_report.evidence[0]
            .findings
            .iter()
            .any(|finding| finding.code == "policy.manifest.payload_schema_mismatch")
    );

    let mut wrong_target = sample_envelope();
    wrong_target.target.role = Some("other".to_string());
    let wrong_target_report = evaluate_manifest_gate_with_envelope(
        &sample_manifest_source("build.json", true),
        wrong_target,
    );

    assert_eq!(
        wrong_target_report.gate_exit_class,
        ExitClassV1::BlockedByPolicy
    );
    assert!(
        wrong_target_report.evidence[0]
            .findings
            .iter()
            .any(|finding| finding.code == "policy.manifest.target_mismatch")
    );
}

#[test]
fn project_evidence_manifest_rejects_duplicate_evidence_paths() {
    let manifest_source = r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "build_provenance"
path = "build.json"
required = true
payload_schema = "canic.build_provenance.v1"

[evidence.target]
fleet = "demo"
role = "app"

[[evidence]]
kind = "deployment_check"
path = " ./build.json "
required = true
payload_schema = "canic.deployment_check.v1"

[evidence.target]
deployment = "demo-staging"
"#;

    let error = parse_project_evidence_manifest_v1(manifest_source)
        .expect_err("duplicate evidence path should fail");

    assert!(matches!(error, PolicyGateError::InvalidPolicy(_)));
    assert!(
        error
            .to_string()
            .contains("duplicates an earlier evidence path")
    );
}
