use super::{
    CiPolicyV1, PolicyEvaluationStatusV1, PolicyFindingV1, PolicyGateError,
    ProjectEvidenceGateEntryReportV1, ProjectEvidenceGateReportV1, ProjectEvidenceManifestEntryV1,
    ProjectEvidenceManifestGateRequest, evaluation::evaluate_policy, parse_ci_policy_v1,
    parse_project_evidence_manifest_v1,
};
use crate::evidence_envelope::{
    EvidenceEnvelopeV1, ExitClassV1, InputFingerprintV1, combine_exit_classes,
    evidence_envelope_schema, file_input_fingerprint, project_evidence_manifest_schema,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn evaluate_project_evidence_manifest_gate(
    request: ProjectEvidenceManifestGateRequest<'_>,
) -> Result<ProjectEvidenceGateReportV1, PolicyGateError> {
    let policy = parse_ci_policy_v1(request.policy_source)?;
    let manifest = parse_project_evidence_manifest_v1(request.manifest_source)?;
    let policy_file_fingerprint = file_input_fingerprint(
        "ci_policy",
        request.policy_path,
        request.fingerprint_root,
        None,
        None,
    )?;
    let manifest_file_fingerprint = file_input_fingerprint(
        "project_evidence_manifest",
        request.manifest_path,
        request.fingerprint_root,
        Some(project_evidence_manifest_schema()),
        None,
    )?;
    let project_root = manifest_project_root(request.manifest_path, &manifest.project.root);
    let mut evidence = Vec::new();

    for entry in &manifest.evidence {
        evidence.push(evaluate_manifest_entry(
            &policy,
            &policy_file_fingerprint,
            &project_root,
            entry,
        )?);
    }

    let has_failures = evidence
        .iter()
        .any(|entry| entry.status == PolicyEvaluationStatusV1::Failed);
    let gate_exit_class = combine_exit_classes(evidence.iter().map(|entry| entry.gate_exit_class));

    Ok(ProjectEvidenceGateReportV1 {
        schema_version: 1,
        manifest_schema_version: manifest.schema_version,
        project_name: manifest.project.name,
        policy_file_fingerprint,
        manifest_file_fingerprint,
        policy_status: if has_failures {
            PolicyEvaluationStatusV1::Failed
        } else {
            PolicyEvaluationStatusV1::Passed
        },
        gate_exit_class,
        evidence,
    })
}

fn evaluate_manifest_entry(
    policy: &CiPolicyV1,
    policy_file_fingerprint: &InputFingerprintV1,
    project_root: &Path,
    entry: &ProjectEvidenceManifestEntryV1,
) -> Result<ProjectEvidenceGateEntryReportV1, PolicyGateError> {
    let evidence_path = resolve_manifest_entry_path(project_root, &entry.path);
    if !evidence_path.is_file() {
        return Ok(missing_manifest_entry_report(entry));
    }

    let envelope_source = fs::read_to_string(&evidence_path)?;
    let envelope = serde_json::from_str::<EvidenceEnvelopeV1>(&envelope_source)?;
    let evaluated_envelope_fingerprint = file_input_fingerprint(
        "evidence_envelope",
        &evidence_path,
        project_root,
        Some(evidence_envelope_schema()),
        None,
    )?;
    let mut policy_report = evaluate_policy(
        policy,
        policy_file_fingerprint.clone(),
        evaluated_envelope_fingerprint.clone(),
        envelope.clone(),
    );
    let mut findings = Vec::new();
    let mut gate_exit_classes = vec![policy_report.gate_exit_class];

    if envelope.payload_schema.id != entry.payload_schema {
        let finding = PolicyFindingV1::error(
            "policy.manifest.payload_schema_mismatch",
            "manifest evidence payload schema does not match the evaluated envelope",
            "manifest.evidence.payload_schema",
            ExitClassV1::BlockedByPolicy,
        )
        .expected(serde_json::json!(entry.payload_schema))
        .actual(serde_json::json!(envelope.payload_schema.id));
        gate_exit_classes.push(finding.exit_class());
        findings.push(finding);
    }

    if !entry.target.matches_envelope_target(&envelope.target) {
        let finding = PolicyFindingV1::error(
            "policy.manifest.target_mismatch",
            "manifest evidence target does not match the evaluated envelope target",
            "manifest.evidence.target",
            ExitClassV1::BlockedByPolicy,
        )
        .expected(serde_json::json!(entry.target))
        .actual(serde_json::json!(envelope.target));
        gate_exit_classes.push(finding.exit_class());
        findings.push(finding);
    }

    policy_report.findings.extend(findings.clone());
    let gate_exit_class = combine_exit_classes(gate_exit_classes);
    policy_report.gate_exit_class = gate_exit_class;
    if !findings.is_empty() {
        policy_report.policy_status = PolicyEvaluationStatusV1::Failed;
    }

    Ok(ProjectEvidenceGateEntryReportV1 {
        kind: entry.kind.clone(),
        path: entry.path.clone(),
        required: entry.required,
        expected_payload_schema: entry.payload_schema.clone(),
        expected_target: entry.target.clone(),
        status: if policy_report.policy_status == PolicyEvaluationStatusV1::Failed {
            PolicyEvaluationStatusV1::Failed
        } else {
            PolicyEvaluationStatusV1::Passed
        },
        gate_exit_class,
        evaluated_envelope_fingerprint: Some(evaluated_envelope_fingerprint),
        policy_report: Some(policy_report),
        findings,
    })
}

fn missing_manifest_entry_report(
    entry: &ProjectEvidenceManifestEntryV1,
) -> ProjectEvidenceGateEntryReportV1 {
    let (status, gate_exit_class, findings) = if entry.required {
        (
            PolicyEvaluationStatusV1::Failed,
            ExitClassV1::MissingRequiredEvidence,
            vec![
                PolicyFindingV1::error(
                    "policy.manifest.required_evidence_missing",
                    "required manifest evidence file is missing",
                    "manifest.evidence.path",
                    ExitClassV1::MissingRequiredEvidence,
                )
                .expected(serde_json::json!(entry.path)),
            ],
        )
    } else {
        (
            PolicyEvaluationStatusV1::Passed,
            ExitClassV1::SuccessWithWarnings,
            vec![
                PolicyFindingV1::warning(
                    "policy.manifest.optional_evidence_missing",
                    "optional manifest evidence file is missing",
                    "manifest.evidence.path",
                )
                .expected(serde_json::json!(entry.path)),
            ],
        )
    };

    ProjectEvidenceGateEntryReportV1 {
        kind: entry.kind.clone(),
        path: entry.path.clone(),
        required: entry.required,
        expected_payload_schema: entry.payload_schema.clone(),
        expected_target: entry.target.clone(),
        status,
        gate_exit_class,
        evaluated_envelope_fingerprint: None,
        policy_report: None,
        findings,
    }
}

fn manifest_project_root(manifest_path: &Path, root: &str) -> PathBuf {
    let root_path = PathBuf::from(root);
    if root_path.is_absolute() {
        return root_path;
    }
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(root_path)
}

fn resolve_manifest_entry_path(project_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}
