use super::*;

#[test]
fn minimal_policy_passes_success_envelope() {
    let root = temp_dir("canic-policy-pass");
    fs::create_dir_all(&root).expect("create root");
    let policy_path = root.join("policy.toml");
    let envelope_path = root.join("envelope.json");
    fs::write(&policy_path, MINIMAL_POLICY).expect("write policy");
    fs::write(&envelope_path, "{}").expect("write envelope placeholder");

    let report = evaluate_policy_gate(PolicyGateRequest {
        policy_source: MINIMAL_POLICY,
        policy_path: &policy_path,
        envelope_path: &envelope_path,
        fingerprint_root: &root,
        envelope: sample_envelope(),
    })
    .expect("evaluate policy");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    assert!(report.findings.is_empty());
    assert_eq!(
        report.evaluated_payload_schema.id,
        "canic.build_provenance.v1"
    );
}

#[test]
fn policy_rejects_disallowed_exit_class_but_preserves_evaluated_class() {
    let mut envelope = sample_envelope();
    envelope.exit_class = ExitClassV1::SuccessWithWarnings;

    let report = evaluate_policy_for_test(MINIMAL_POLICY, envelope);

    assert_eq!(
        report.evaluated_envelope_exit_class,
        ExitClassV1::SuccessWithWarnings
    );
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Failed);
    assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    assert_eq!(report.findings[0].code, "policy.exit_class.disallowed");
}

#[test]
fn policy_accepts_success_with_warnings_when_allowed() {
    let mut envelope = sample_envelope();
    envelope.exit_class = ExitClassV1::SuccessWithWarnings;
    envelope.summary.warnings.push(EvidenceMessageV1::new(
        "test.warning",
        "warning",
        EvidenceMessageSeverityV1::Warning,
    ));

    let report = evaluate_policy_for_test(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success", "success_with_warnings"]
"#,
        envelope,
    );

    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::SuccessWithWarnings);
}

#[test]
fn summary_conflicts_and_missing_required_inputs_map_to_policy_exit_classes() {
    let mut conflict = sample_envelope();
    conflict
        .summary
        .evidence_conflicts
        .push(EvidenceMessageV1::new(
            "test.conflict",
            "conflict",
            EvidenceMessageSeverityV1::Error,
        ));
    let conflict_report = evaluate_policy_for_test(SUMMARY_POLICY, conflict);

    assert_eq!(
        conflict_report.gate_exit_class,
        ExitClassV1::EvidenceConflict
    );
    assert_eq!(
        conflict_report.findings[0].code,
        "policy.summary.evidence_conflict"
    );

    let missing_report = evaluate_policy_for_test(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[[required_input]]
kind = "canic_config"
schema = "canic.config.toml"
"#,
        sample_envelope(),
    );

    assert_eq!(
        missing_report.gate_exit_class,
        ExitClassV1::MissingRequiredEvidence
    );
    assert_eq!(
        missing_report.findings[0].code,
        "policy.required_input.missing"
    );
}

#[test]
fn required_input_passes_on_matching_kind_and_schema() {
    let mut envelope = sample_envelope();
    envelope.inputs.push(InputFingerprintV1 {
        kind: "canic_config".to_string(),
        path: Some("canic.toml".to_string()),
        path_display: InputPathDisplayV1::Relative,
        sha256: None,
        size_bytes: None,
        modified_unix_secs: None,
        schema: Some(PayloadSchemaRefV1::stable("canic.config.toml", "1")),
        note: None,
    });

    let report = evaluate_policy_for_test(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[[required_input]]
kind = "canic_config"
schema = "canic.config.toml"
"#,
        envelope,
    );

    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
}
