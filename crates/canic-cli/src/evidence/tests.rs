//! Module: evidence::tests
//!
//! Responsibility: verify `canic evidence` parsing, comparison, and gate behavior.
//! Does not own: production dispatch or policy-gate implementation.
//! Boundary: in-module tests for the evidence CLI facade and submodules.

use super::{
    EvidenceCommandError,
    command::gate_usage,
    compare::{EvidenceCompareStatus, compare_envelope_files, compare_envelopes},
    gate::{EvidenceGateReport, evaluate_gate_files, policy_gate_envelope, write_gate_report},
    options::{
        EvidenceCompareFormat, EvidenceCompareOptions, EvidenceGateFormat, EvidenceGateInput,
        EvidenceGateOptions,
    },
};
use crate::test_support::temp_dir;
use canic_host::evidence_envelope::{
    CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
    EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, adoption_report_schema,
    evidence_envelope_schema,
};
use canic_host::policy_gate::PolicyEvaluationStatusV1;
use serde_json::json;
use std::{ffi::OsString, fs, path::PathBuf, time::UNIX_EPOCH};

#[test]
fn parses_compare_options() {
    let options = EvidenceCompareOptions::parse([
        OsString::from("--left"),
        OsString::from("left.json"),
        OsString::from("--right"),
        OsString::from("right.json"),
        OsString::from("--json"),
    ])
    .expect("parse options");

    assert_eq!(options.left, PathBuf::from("left.json"));
    assert_eq!(options.right, PathBuf::from("right.json"));
    assert_eq!(options.format, EvidenceCompareFormat::Json);
}

#[test]
fn parses_compare_options_default_text() {
    let options = EvidenceCompareOptions::parse([
        OsString::from("--left"),
        OsString::from("left.json"),
        OsString::from("--right"),
        OsString::from("right.json"),
    ])
    .expect("parse options");

    assert_eq!(options.format, EvidenceCompareFormat::Text);
}

#[test]
fn parses_gate_options() {
    let options = EvidenceGateOptions::parse([
        OsString::from("--policy"),
        OsString::from("policy.toml"),
        OsString::from("--envelope"),
        OsString::from("evidence.json"),
        OsString::from("--evidence-envelope"),
        OsString::from("--output"),
        OsString::from("gate.json"),
    ])
    .expect("parse options");

    assert_eq!(options.policy, PathBuf::from("policy.toml"));
    assert_eq!(
        options.input,
        EvidenceGateInput::Envelope(PathBuf::from("evidence.json"))
    );
    assert_eq!(options.format, EvidenceGateFormat::EnvelopeJson);
    assert_eq!(options.output, Some(PathBuf::from("gate.json")));
}

#[test]
fn parses_gate_manifest_options() {
    let options = EvidenceGateOptions::parse([
        OsString::from("--policy"),
        OsString::from("policy.toml"),
        OsString::from("--manifest"),
        OsString::from("evidence.toml"),
    ])
    .expect("parse options");

    assert_eq!(
        options.input,
        EvidenceGateInput::Manifest(PathBuf::from("evidence.toml"))
    );
    assert_eq!(options.format, EvidenceGateFormat::Text);
}

#[test]
fn parses_gate_json_options() {
    let options = EvidenceGateOptions::parse([
        OsString::from("--policy"),
        OsString::from("policy.toml"),
        OsString::from("--envelope"),
        OsString::from("evidence.json"),
        OsString::from("--json"),
    ])
    .expect("parse options");

    assert_eq!(options.format, EvidenceGateFormat::Json);
}

#[test]
fn gate_help_shows_v1_manifest_and_envelope_examples() {
    let text = gate_usage();

    assert!(text.contains("Usage: canic evidence gate"));
    assert!(text.contains("canic evidence gate --policy ci/canic-policy.toml --envelope"));
    assert!(text.contains("canic evidence gate --policy ci/canic-policy.toml --manifest"));
    assert!(text.contains("does not run builds, deploy, discover live state"));
}

#[test]
fn gate_rejects_envelope_and_manifest_together() {
    let err = EvidenceGateOptions::parse([
        OsString::from("--policy"),
        OsString::from("policy.toml"),
        OsString::from("--envelope"),
        OsString::from("evidence.json"),
        OsString::from("--manifest"),
        OsString::from("evidence.toml"),
    ])
    .expect_err("conflicting gate inputs fail");

    assert!(matches!(err, EvidenceCommandError::Usage(_)));
}

#[test]
fn compare_ignores_timestamp_version_and_payload_body() {
    let left_path = PathBuf::from("left.json");
    let right_path = PathBuf::from("right.json");
    let mut left = sample_envelope();
    let mut right = sample_envelope();
    right.canic_version = "different".to_string();
    right.generated_at = "2030-01-01T00:00:00Z".to_string();
    right.payload = json!({ "changed": true });

    let report = compare_envelopes(&left, &right, &left_path, &right_path);

    assert_eq!(report.status, EvidenceCompareStatus::Matched);
    assert!(report.differences.is_empty());
    left.payload_sha256 = Some("left-hash".to_string());
    right.payload_sha256 = Some("right-hash".to_string());

    let report = compare_envelopes(&left, &right, &left_path, &right_path);

    assert_eq!(report.status, EvidenceCompareStatus::Different);
    assert_eq!(report.differences[0].field, "payload_sha256");
}

#[test]
fn compare_detects_exit_class_and_summary_differences() {
    let left_path = PathBuf::from("left.json");
    let right_path = PathBuf::from("right.json");
    let left = sample_envelope();
    let mut right = sample_envelope();
    right.exit_class = ExitClassV1::EvidenceConflict;
    right
        .summary
        .evidence_conflicts
        .push(EvidenceMessageV1::new(
            "test.conflict",
            "conflict",
            EvidenceMessageSeverityV1::Error,
        ));

    let report = compare_envelopes(&left, &right, &left_path, &right_path);

    assert_eq!(report.status, EvidenceCompareStatus::Different);
    assert!(
        report
            .differences
            .iter()
            .any(|difference| difference.field == "exit_class")
    );
    assert!(
        report
            .differences
            .iter()
            .any(|difference| difference.field == "summary")
    );
}

#[test]
fn compare_files_returns_error_for_differences_after_writing_report() {
    let left_path = temp_json_path("canic-evidence-left");
    let right_path = temp_json_path("canic-evidence-right");
    let left = sample_envelope();
    let mut right = sample_envelope();
    right.exit_class = ExitClassV1::BlockedByPolicy;
    fs::write(&left_path, serde_json::to_vec(&left).expect("encode left")).expect("write left");
    fs::write(
        &right_path,
        serde_json::to_vec(&right).expect("encode right"),
    )
    .expect("write right");
    let options = EvidenceCompareOptions {
        left: left_path.clone(),
        right: right_path.clone(),
        format: EvidenceCompareFormat::Text,
    };

    let report = compare_envelope_files(&options).expect("compare files");

    fs::remove_file(left_path).expect("clean left");
    fs::remove_file(right_path).expect("clean right");
    assert_eq!(report.status, EvidenceCompareStatus::Different);
    assert_eq!(report.differences[0].field, "exit_class");
}

#[test]
fn gate_files_evaluate_policy_and_json_output() {
    let root = temp_dir("canic-evidence-gate-json");
    fs::create_dir_all(&root).expect("create root");
    let policy = root.join("policy.toml");
    let envelope = root.join("envelope.json");
    let output = root.join("gate.json");
    fs::write(&policy, MINIMAL_POLICY).expect("write policy");
    fs::write(
        &envelope,
        serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
    )
    .expect("write envelope");
    let options = EvidenceGateOptions {
        policy,
        input: EvidenceGateInput::Envelope(envelope),
        format: EvidenceGateFormat::Json,
        output: Some(output.clone()),
    };

    let report = evaluate_gate_files(&options).expect("evaluate gate");
    write_gate_report(&options, &report).expect("write gate");
    let written = fs::read_to_string(&output).expect("read output");

    fs::remove_dir_all(root).expect("clean");
    let EvidenceGateReport::Envelope(report) = report else {
        panic!("expected single envelope gate report");
    };
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    assert!(written.contains("\"policy_status\": \"passed\""));
}

#[test]
fn gate_envelope_wraps_policy_report() {
    let root = temp_dir("canic-evidence-gate-envelope");
    fs::create_dir_all(&root).expect("create root");
    let policy = root.join("policy.toml");
    let envelope = root.join("envelope.json");
    fs::write(&policy, MINIMAL_POLICY).expect("write policy");
    fs::write(
        &envelope,
        serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
    )
    .expect("write envelope");
    let options = EvidenceGateOptions {
        policy,
        input: EvidenceGateInput::Envelope(envelope),
        format: EvidenceGateFormat::EnvelopeJson,
        output: None,
    };
    let report = evaluate_gate_files(&options).expect("evaluate gate");

    let gate = policy_gate_envelope(&options, &report).expect("wrap gate");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(gate.target.kind, EvidenceTargetKindV1::PolicyGate);
    assert_eq!(gate.target.app.as_deref(), Some("demo"));
    assert_eq!(gate.payload_schema.id, "canic.policy_gate_report.v1");
    assert_eq!(gate.exit_class, ExitClassV1::Success);
    assert_eq!(gate.inputs.len(), 2);
}

#[test]
fn gate_manifest_evaluates_project_evidence_and_wraps_report() {
    let root = temp_dir("canic-evidence-gate-manifest");
    fs::create_dir_all(&root).expect("create root");
    let policy = root.join("policy.toml");
    let manifest = root.join("evidence.toml");
    let envelope = root.join("adoption.json");
    fs::write(&policy, MINIMAL_POLICY).expect("write policy");
    fs::write(
        &envelope,
        serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
    )
    .expect("write envelope");
    fs::write(&manifest, sample_manifest()).expect("write manifest");
    let options = EvidenceGateOptions {
        policy,
        input: EvidenceGateInput::Manifest(manifest),
        format: EvidenceGateFormat::EnvelopeJson,
        output: None,
    };

    let report = evaluate_gate_files(&options).expect("evaluate manifest gate");
    let envelope = policy_gate_envelope(&options, &report).expect("wrap manifest gate");

    fs::remove_dir_all(root).expect("clean");
    let EvidenceGateReport::Manifest(report) = report else {
        panic!("expected manifest gate report");
    };
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
    assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    assert_eq!(report.evidence.len(), 1);
    assert_eq!(
        envelope.payload_schema.id,
        "canic.project_evidence_gate_report.v1"
    );
    assert_eq!(envelope.target.profile.as_deref(), Some("demo"));
    assert_eq!(envelope.inputs.len(), 3);
}

#[test]
fn gate_writes_report_before_failure_can_be_returned() {
    let root = temp_dir("canic-evidence-gate-failure");
    fs::create_dir_all(&root).expect("create root");
    let policy = root.join("policy.toml");
    let envelope = root.join("envelope.json");
    let output = root.join("gate.json");
    let mut failing = sample_envelope();
    failing.exit_class = ExitClassV1::SuccessWithWarnings;
    fs::write(&policy, MINIMAL_POLICY).expect("write policy");
    fs::write(
        &envelope,
        serde_json::to_vec(&failing).expect("encode envelope"),
    )
    .expect("write envelope");
    let options = EvidenceGateOptions {
        policy,
        input: EvidenceGateInput::Envelope(envelope),
        format: EvidenceGateFormat::Json,
        output: Some(output.clone()),
    };

    let report = evaluate_gate_files(&options).expect("evaluate gate");
    write_gate_report(&options, &report).expect("write failing report");
    let written = fs::read_to_string(&output).expect("read output");

    fs::remove_dir_all(root).expect("clean");
    let EvidenceGateReport::Envelope(report) = report else {
        panic!("expected single envelope gate report");
    };
    assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Failed);
    assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    assert!(written.contains("\"gate_exit_class\": \"blocked_by_policy\""));
}

fn sample_envelope() -> EvidenceEnvelopeV1 {
    EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: CommandProvenanceV1 {
            name: "canic app adoption report".to_string(),
            argv_normalized: vec![
                "canic".to_string(),
                "app".to_string(),
                "adoption".to_string(),
                "report".to_string(),
                "demo".to_string(),
            ],
            argv_redactions: Vec::new(),
            format: "envelope-json".to_string(),
        },
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::AppAdoption,
            deployment: None,
            app: Some("demo".to_string()),
            fleet: None,
            role: None,
            profile: Some("minimal".to_string()),
            environment: None,
        },
        generated_at: "2026-05-31T00:00:00Z".to_string(),
        source_config: None,
        inputs: Vec::new(),
        payload_schema: adoption_report_schema(),
        payload_sha256: Some(sample_sha256("payload")),
        payload: json!({ "report_id": "report-1" }),
        summary: EvidenceSummaryV1 {
            warnings: Vec::new(),
            blocked_actions: Vec::new(),
            missing_or_stale_evidence: Vec::new(),
            evidence_conflicts: Vec::new(),
        },
        exit_class: ExitClassV1::Success,
    }
}

fn sample_sha256(label: &str) -> String {
    let mut hash = String::new();
    while hash.len() < 64 {
        hash.push_str(label);
    }
    hash.truncate(64);
    hash
}

fn sample_manifest() -> String {
    r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "adoption_report"
path = "adoption.json"
required = true
payload_schema = "canic.adoption_report.v1"

[evidence.target]
app = "demo"
profile = "minimal"
"#
    .to_string()
}

fn temp_json_path(name: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{suffix}.json"))
}

const MINIMAL_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#;
