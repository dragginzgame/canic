use super::model::PolicyBuildProvenanceRuleV1;
use super::*;
use crate::build_provenance::{
    ArtifactProvenanceKindV1, ArtifactProvenanceV1, ArtifactTransformKindV1,
    ArtifactTransformModeV1, ArtifactTransformOutcomeV1, ArtifactTransformProvenanceV1,
    BuildProvenanceStatusV1, BuildProvenanceV1, BuildScriptInputStateV1, CargoProvenanceV1,
    SourceDirtyPolicyV1, SourceProvenanceV1, SourceVcsV1,
};
use crate::evidence_envelope::{
    CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
    EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
    InputPathDisplayV1, PayloadSchemaRefV1, PayloadSchemaStabilityV1, evidence_envelope_schema,
    policy_gate_report_schema,
};
use crate::test_support::temp_dir;
use serde_json::json;
use std::fs;

mod build_provenance;
mod envelope;
mod manifest;
mod parser;
mod schema;

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
        transforms: vec![ArtifactTransformProvenanceV1 {
            role: "app".to_string(),
            transform: ArtifactTransformKindV1::Shrink,
            mode: ArtifactTransformModeV1::Optional,
            tool: "ic-wasm".to_string(),
            tool_version: Some("ic-wasm 0.test".to_string()),
            outcome: ArtifactTransformOutcomeV1::Applied,
        }],
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
