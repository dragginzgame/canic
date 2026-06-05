use super::*;
use crate::build_provenance::{
    ArtifactProvenanceV1, BuildProvenanceStatusV1, BuildScriptInputStateV1, CargoProvenanceV1,
    SourceProvenanceV1, SourceVcsV1,
};
use crate::evidence_envelope::{
    CommandProvenanceV1, EvidenceMessageSeverityV1, EvidenceMessageV1, EvidenceTargetKindV1,
    InputPathDisplayV1, PayloadSchemaStabilityV1, evidence_envelope_schema,
    policy_gate_report_schema,
};
use crate::test_support::temp_dir;
use serde_json::json;
use std::fs;

#[test]
fn policy_parser_accepts_minimal_policy() {
    let policy = parse_ci_policy_v1(MINIMAL_POLICY).expect("parse policy");

    assert_eq!(policy.schema_version, 1);
    assert_eq!(
        policy.envelope.required_schema,
        "canic.evidence_envelope.v1"
    );
    assert_eq!(policy.exit_class.allowed, vec![ExitClassV1::Success]);
}

#[test]
fn policy_parser_rejects_unknown_keys_and_empty_allow_lists() {
    let unknown = parse_ci_policy_v1(
        r#"
schema_version = 1
unexpected = true

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#,
    )
    .expect_err("unknown policy keys fail");
    assert!(unknown.to_string().contains("failed to parse policy TOML"));

    let empty = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = []
"#,
    )
    .expect_err("empty allow list fails");
    assert!(empty.to_string().contains("exit_class.allowed"));
}

#[test]
fn policy_parser_accepts_build_provenance_rules() {
    let policy = parse_ci_policy_v1(BUILD_PROVENANCE_POLICY).expect("parse policy");

    let rules = policy
        .build_provenance
        .expect("build provenance rules present");
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CleanSource));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CargoLock));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::WasmGzip));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::Sha256));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget));
}

#[test]
fn policy_parser_rejects_empty_build_provenance_rules() {
    let err = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
"#,
    )
    .expect_err("empty build provenance rules fail");

    assert!(err.to_string().contains("build_provenance"));
}

#[test]
fn policy_parser_rejects_unknown_build_provenance_keys() {
    let err = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
require_magic = true
"#,
    )
    .expect_err("unknown build provenance keys fail");

    assert!(err.to_string().contains("failed to parse policy TOML"));
}

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

#[test]
fn policy_gate_report_schema_is_stable() {
    assert_eq!(
        policy_gate_report_schema(),
        PayloadSchemaRefV1 {
            id: "canic.policy_gate_report.v1".to_string(),
            version: "1".to_string(),
            stability: PayloadSchemaStabilityV1::Stable,
        }
    );
}

fn evaluate_policy_for_test(
    policy_source: &str,
    envelope: EvidenceEnvelopeV1,
) -> PolicyGateReportV1 {
    let root = temp_dir("canic-policy-test");
    fs::create_dir_all(&root).expect("create root");
    let policy_path = root.join("policy.toml");
    let envelope_path = root.join("envelope.json");
    fs::write(&policy_path, policy_source).expect("write policy");
    fs::write(&envelope_path, "{}").expect("write envelope placeholder");

    let report = evaluate_policy_gate(PolicyGateRequest {
        policy_source,
        policy_path: &policy_path,
        envelope_path: &envelope_path,
        fingerprint_root: &root,
        envelope,
    })
    .expect("evaluate policy");

    fs::remove_dir_all(root).expect("clean");
    report
}

fn evaluate_manifest_gate_for_test(
    manifest_source: &str,
    policy_source: &str,
) -> ProjectEvidenceGateReportV1 {
    let root = temp_dir("canic-policy-manifest-test");
    fs::create_dir_all(&root).expect("create root");
    let policy_path = root.join("policy.toml");
    let manifest_path = root.join("evidence.toml");
    fs::write(&policy_path, policy_source).expect("write policy");
    fs::write(&manifest_path, manifest_source).expect("write manifest");

    let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
        policy_source,
        policy_path: &policy_path,
        manifest_source,
        manifest_path: &manifest_path,
        fingerprint_root: &root,
    })
    .expect("evaluate manifest gate");

    fs::remove_dir_all(root).expect("clean");
    report
}

fn evaluate_manifest_gate_with_envelope(
    manifest_source: &str,
    envelope: EvidenceEnvelopeV1,
) -> ProjectEvidenceGateReportV1 {
    let root = temp_dir("canic-policy-manifest-envelope-test");
    fs::create_dir_all(&root).expect("create root");
    let policy_path = root.join("policy.toml");
    let manifest_path = root.join("evidence.toml");
    let envelope_path = root.join("build.json");
    fs::write(&policy_path, BUILD_PROVENANCE_POLICY).expect("write policy");
    fs::write(&manifest_path, manifest_source).expect("write manifest");
    fs::write(
        &envelope_path,
        serde_json::to_vec(&envelope).expect("encode envelope"),
    )
    .expect("write envelope");

    let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
        policy_source: BUILD_PROVENANCE_POLICY,
        policy_path: &policy_path,
        manifest_source,
        manifest_path: &manifest_path,
        fingerprint_root: &root,
    })
    .expect("evaluate manifest gate");

    fs::remove_dir_all(root).expect("clean");
    report
}

fn sample_envelope() -> EvidenceEnvelopeV1 {
    sample_envelope_with_payload(
        serde_json::to_value(sample_build_provenance_payload()).expect("payload json"),
    )
}

fn sample_envelope_with_payload(payload: serde_json::Value) -> EvidenceEnvelopeV1 {
    EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: CommandProvenanceV1 {
            name: "canic build".to_string(),
            argv_normalized: Vec::new(),
            argv_redactions: Vec::new(),
            format: "envelope-json".to_string(),
        },
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::Artifact,
            deployment: None,
            fleet: Some("demo".to_string()),
            role: Some("app".to_string()),
            profile: None,
            network: None,
        },
        generated_at: "unix:1".to_string(),
        source_config: None,
        inputs: Vec::new(),
        payload_schema: PayloadSchemaRefV1::stable("canic.build_provenance.v1", "1"),
        payload_sha256: Some("0".repeat(64)),
        payload,
        summary: EvidenceSummaryV1 {
            warnings: Vec::new(),
            blocked_actions: Vec::new(),
            missing_or_stale_evidence: Vec::new(),
            evidence_conflicts: Vec::new(),
        },
        exit_class: ExitClassV1::Success,
    }
}

fn sample_build_provenance_payload() -> BuildProvenanceV1 {
    BuildProvenanceV1 {
        schema_version: 1,
        generated_at: "unix:1".to_string(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: CommandProvenanceV1 {
            name: "canic build".to_string(),
            argv_normalized: vec![
                "canic".to_string(),
                "build".to_string(),
                "demo".to_string(),
                "app".to_string(),
            ],
            argv_redactions: Vec::new(),
            format: "provenance".to_string(),
        },
        build_status: BuildProvenanceStatusV1::Success,
        source: SourceProvenanceV1 {
            schema_version: 1,
            vcs: SourceVcsV1::Git,
            revision: Some("abc123".to_string()),
            branch: Some("main".to_string()),
            dirty: Some(false),
            dirty_policy: SourceDirtyPolicyV1::Clean,
            dirty_summary_digest: None,
            dirty_summary_algorithm: None,
        },
        cargo: CargoProvenanceV1 {
            cargo_lock_sha256: Some("1".repeat(64)),
            package_manifest_sha256: Some("2".repeat(64)),
            package_name: "demo_app".to_string(),
            package_manifest: "fleets/demo/app/Cargo.toml".to_string(),
            package_metadata_fleet: "demo".to_string(),
            package_metadata_role: "app".to_string(),
            rustc_version: Some("rustc 1.88.0".to_string()),
            cargo_version: Some("cargo 1.88.0".to_string()),
            target: Some("wasm32-unknown-unknown".to_string()),
            profile: "fast".to_string(),
            features: Vec::new(),
            default_features: None,
            rustflags_digest: None,
            rustflags_digest_algorithm: None,
            cargo_config_fingerprints: Vec::new(),
            build_script_inputs: BuildScriptInputStateV1::NotRecorded,
        },
        artifacts: vec![
            sample_artifact(ArtifactProvenanceKindV1::Wasm, "a"),
            sample_artifact(ArtifactProvenanceKindV1::WasmGzip, "b"),
        ],
        warnings: Vec::new(),
    }
}

fn sample_artifact(kind: ArtifactProvenanceKindV1, hash_char: &str) -> ArtifactProvenanceV1 {
    ArtifactProvenanceV1 {
        role: "app".to_string(),
        fleet: "demo".to_string(),
        artifact_kind: kind,
        path: Some("target/app.wasm.gz".to_string()),
        path_display: InputPathDisplayV1::Relative,
        hash_algorithm: "sha256".to_string(),
        sha256: hash_char.repeat(64),
        size_bytes: 123,
        produced_by: "canic build".to_string(),
    }
}

fn sample_manifest_source(path: &str, required: bool) -> String {
    format!(
        r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "build_provenance"
path = "{path}"
required = {required}
payload_schema = "canic.build_provenance.v1"

[evidence.target]
fleet = "demo"
role = "app"
"#
    )
}

const MINIMAL_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#;

const SUMMARY_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success", "success_with_warnings"]

[summary]
fail_on_evidence_conflicts = true
fail_on_blocked_actions = true
allow_missing_or_stale_evidence = false
"#;

const BUILD_PROVENANCE_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
require_clean_source = true
require_cargo_lock = true
require_wasm_gzip = true
require_sha256 = true
require_package_identity_matches_target = true
"#;
