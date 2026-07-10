use super::*;
use std::path::PathBuf;

#[test]
fn exit_class_serializes_to_snake_case() {
    let encoded = serde_json::to_string(&ExitClassV1::SuccessWithWarnings).expect("serialize");

    assert_eq!(encoded, "\"success_with_warnings\"");
    assert_eq!(
        ExitClassV1::SuccessWithWarnings.label(),
        "success_with_warnings"
    );
    assert_eq!(
        ExitClassV1::from_label("success_with_warnings"),
        Some(ExitClassV1::SuccessWithWarnings)
    );
    assert_eq!(ExitClassV1::from_label("success-ish"), None);
}

#[test]
fn exit_class_precedence_prefers_policy_relevant_failures() {
    assert_eq!(
        combine_exit_classes([
            ExitClassV1::SuccessWithWarnings,
            ExitClassV1::BlockedByPolicy,
            ExitClassV1::EvidenceConflict,
        ]),
        ExitClassV1::EvidenceConflict
    );
    assert!(ExitClassV1::InvalidInput.dominates(ExitClassV1::EvidenceConflict));
    assert!(ExitClassV1::InternalError.dominates(ExitClassV1::ExecutionFailed));
}

#[test]
fn evidence_summary_exit_class_uses_stable_precedence() {
    let mut summary = EvidenceSummaryV1 {
        warnings: vec![EvidenceMessageV1::new(
            "test.warning",
            "warning",
            EvidenceMessageSeverityV1::Warning,
        )],
        blocked_actions: Vec::new(),
        missing_or_stale_evidence: Vec::new(),
        evidence_conflicts: Vec::new(),
    };

    assert_eq!(
        evidence_summary_exit_class(&summary, false),
        ExitClassV1::SuccessWithWarnings
    );

    summary.blocked_actions.push(EvidenceMessageV1::new(
        "test.blocked",
        "blocked",
        EvidenceMessageSeverityV1::Error,
    ));
    assert_eq!(
        evidence_summary_exit_class(&summary, false),
        ExitClassV1::BlockedByPolicy
    );
    assert_eq!(
        evidence_summary_exit_class(&summary, true),
        ExitClassV1::MissingRequiredEvidence
    );

    summary.evidence_conflicts.push(EvidenceMessageV1::new(
        "test.conflict",
        "conflict",
        EvidenceMessageSeverityV1::Error,
    ));
    assert_eq!(
        evidence_summary_exit_class(&summary, true),
        ExitClassV1::EvidenceConflict
    );
}

#[test]
fn schema_refs_record_stability() {
    assert_eq!(
        evidence_envelope_schema(),
        PayloadSchemaRefV1 {
            id: "canic.evidence_envelope.v1".to_string(),
            version: "1".to_string(),
            stability: PayloadSchemaStabilityV1::Stable,
        }
    );
    assert_eq!(
        adoption_report_schema().stability,
        PayloadSchemaStabilityV1::Experimental
    );
    assert_eq!(
        deployment_check_schema().stability,
        PayloadSchemaStabilityV1::Internal
    );
}

#[test]
fn file_input_fingerprint_uses_relative_path_under_root() {
    let root = temp_dir("canic-envelope-relative");
    let input = root.join("evidence").join("input.json");
    fs::create_dir_all(input.parent().expect("input parent")).expect("create parent");
    fs::write(&input, b"{\"ok\":true}").expect("write input");

    let fingerprint =
        file_input_fingerprint("input", &input, &root, None, None).expect("fingerprint");

    fs::remove_dir_all(&root).expect("clean temp dir");
    assert_eq!(fingerprint.path.as_deref(), Some("evidence/input.json"));
    assert_eq!(fingerprint.path_display, InputPathDisplayV1::Relative);
    assert_eq!(fingerprint.size_bytes, Some(11));
    assert!(
        fingerprint
            .sha256
            .as_deref()
            .is_some_and(|hash| hash.len() == 64)
    );
}

#[test]
fn file_input_fingerprint_redacts_absolute_path_outside_root() {
    let root = temp_dir("canic-envelope-root");
    let outside = temp_dir("canic-envelope-outside");
    fs::create_dir_all(&root).expect("create root");
    fs::create_dir_all(&outside).expect("create outside");
    let input = outside.join("secret.json");
    fs::write(&input, b"secret").expect("write input");

    let fingerprint =
        file_input_fingerprint("input", &input, &root, None, None).expect("fingerprint");
    let command_path = command_path_for_root(&input, &root);

    fs::remove_dir_all(&root).expect("clean root");
    fs::remove_dir_all(&outside).expect("clean outside");
    assert_eq!(fingerprint.path, None);
    assert_eq!(
        fingerprint.path_display,
        InputPathDisplayV1::AbsoluteRedacted
    );
    assert_eq!(command_path, "<redacted:absolute-outside-root>");
}

fn temp_dir(name: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{suffix}"))
}
