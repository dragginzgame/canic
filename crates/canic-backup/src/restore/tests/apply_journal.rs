use super::*;

// Ensure command output receipts keep bounded tail output and byte counts.
#[test]
fn apply_command_output_bounds_to_tail_bytes() {
    let output = RestoreApplyCommandOutput::from_bytes(b"abcdef", 3);

    assert_eq!(output.text, "def");
    assert!(output.truncated);
    assert_eq!(output.original_bytes, 6);
}
// Ensure an artifact-validated apply dry-run produces a ready initial journal.
#[test]
fn apply_journal_marks_validated_operations_ready() {
    let root = temp_dir("canic-restore-apply-journal-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.journal_version, 1);
    assert_eq!(journal.backup_id.as_str(), "fbk_test_001");
    assert!(journal.ready);
    assert!(journal.blocked_reasons.is_empty());
    assert_eq!(journal.operation_count, 6);
    assert_eq!(journal.ready_operations, 6);
    assert_eq!(journal.blocked_operations, 0);
    assert_eq!(journal.operations[0].sequence, 0);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(journal.operations[0].blocking_reasons.is_empty());
}

// Ensure apply journals block when artifact validation was not supplied.
#[test]
fn apply_journal_blocks_without_artifact_validation() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    assert!(!journal.ready);
    assert_eq!(journal.ready_operations, 0);
    assert_eq!(journal.blocked_operations, 6);
    assert!(
        journal
            .blocked_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
    assert!(
        journal.operations[0]
            .blocking_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
}

// Ensure apply journal status exposes compact readiness and next-operation state.
#[test]
fn apply_journal_status_reports_next_ready_operation() {
    let root = temp_dir("canic-restore-apply-journal-status");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let status = journal.status();
    let report = journal.report();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(status.status_version, 1);
    assert_eq!(status.backup_id.as_str(), "fbk_test_001");
    assert!(status.ready);
    assert!(!status.complete);
    assert_eq!(status.operation_count, 6);
    assert_eq!(status.operation_counts.snapshot_uploads, 2);
    assert_eq!(status.operation_counts.snapshot_loads, 2);
    assert_eq!(status.operation_counts.code_reinstalls, 0);
    assert_eq!(status.operation_counts.member_verifications, 2);
    assert_eq!(status.operation_counts.fleet_verifications, 0);
    assert_eq!(status.operation_counts.verification_operations, 2);
    assert!(status.operation_counts_supplied);
    assert_eq!(journal.operation_counts, status.operation_counts);
    assert_eq!(report.operation_counts, status.operation_counts);
    assert!(report.operation_counts_supplied);
    assert_eq!(status.progress.operation_count, 6);
    assert_eq!(status.progress.completed_operations, 0);
    assert_eq!(status.progress.remaining_operations, 6);
    assert_eq!(status.progress.transitionable_operations, 6);
    assert_eq!(status.progress.attention_operations, 0);
    assert_eq!(status.progress.completion_basis_points, 0);
    assert_eq!(report.progress, status.progress);
    assert_eq!(status.pending_summary.pending_operations, 0);
    assert!(!status.pending_summary.pending_operation_available);
    assert_eq!(status.pending_summary.pending_sequence, None);
    assert_eq!(status.pending_summary.pending_operation, None);
    assert_eq!(status.pending_summary.pending_updated_at, None);
    assert!(!status.pending_summary.pending_updated_at_known);
    assert_eq!(report.pending_summary, status.pending_summary);
    assert_eq!(status.ready_operations, 6);
    assert_eq!(status.next_ready_sequence, Some(0));
    assert_eq!(
        status.next_ready_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Ready)
    );
    assert_eq!(
        status.next_transition_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
}

// Ensure next-operation output exposes the full next ready journal row.
#[test]
fn apply_journal_next_operation_reports_full_ready_row() {
    let root = temp_dir("canic-restore-apply-journal-next");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark operation completed");
    let next = journal.next_operation();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(next.ready);
    assert!(!next.complete);
    assert!(next.operation_available);
    let operation = next.operation.expect("next operation");
    assert_eq!(operation.sequence, 1);
    assert_eq!(operation.state, RestoreApplyOperationState::Ready);
    assert_eq!(operation.operation, RestoreApplyOperationKind::LoadSnapshot);
    assert_eq!(operation.source_canister, ROOT);
}

// Ensure blocked journals report no next ready operation.
#[test]
fn apply_journal_next_operation_reports_blocked_state() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let next = journal.next_operation();

    assert!(!next.ready);
    assert!(!next.operation_available);
    assert!(next.operation.is_none());
    assert!(
        next.blocked_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
}

// Ensure command previews expose the dfx upload command without executing it.
#[test]
fn apply_journal_command_preview_reports_upload_command() {
    let root = temp_dir("canic-restore-apply-command-upload");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview();
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.ready);
    assert!(preview.operation_available);
    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "dfx");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            "--dir".to_string(),
            expected_artifact_path,
            ROOT.to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure command previews carry configured dfx program and network.
#[test]
fn apply_journal_command_preview_honors_command_config() {
    let root = temp_dir("canic-restore-apply-command-config");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
        program: "/tmp/dfx".to_string(),
        network: Some("local".to_string()),
    });
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "/tmp/dfx");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "--network".to_string(),
            "local".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            "--dir".to_string(),
            expected_artifact_path,
            ROOT.to_string(),
        ]
    );
}

// Ensure command previews expose stopped-canister hints for snapshot load.
#[test]
fn apply_journal_command_preview_reports_load_command() {
    let root = temp_dir("canic-restore-apply-command-load");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark upload completed");
    journal
        .record_operation_receipt(RestoreApplyOperationReceipt::completed_upload(
            &journal.operations[0],
            "target-snap-root".to_string(),
        ))
        .expect("record upload receipt");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "load".to_string(),
            ROOT.to_string(),
            "target-snap-root".to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(command.requires_stopped_canister);
}

// Ensure load commands cannot render until upload receipts provide target IDs.
#[test]
fn apply_journal_load_command_requires_uploaded_snapshot_receipt() {
    let root = temp_dir("canic-restore-apply-command-load-receipt");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed(0)
        .expect("mark upload completed");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.operation_available);
    assert!(!preview.command_available);
    assert_eq!(
        preview
            .operation
            .expect("next operation should be load")
            .operation,
        RestoreApplyOperationKind::LoadSnapshot
    );
}

// Ensure command previews expose reinstall commands without executing them.
#[test]
fn apply_journal_command_preview_reports_reinstall_command() {
    let journal = command_preview_journal(RestoreApplyOperationKind::ReinstallCode, None, None);
    let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
        program: "dfx".to_string(),
        network: Some("local".to_string()),
    });

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "--network".to_string(),
            "local".to_string(),
            "install".to_string(),
            "--mode".to_string(),
            "reinstall".to_string(),
            "--yes".to_string(),
            ROOT.to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure status verification previews use `dfx canister status`.
#[test]
fn apply_journal_command_preview_reports_status_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("status"),
        None,
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "status".to_string(),
            ROOT.to_string()
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure method verification previews use `dfx canister call`.
#[test]
fn apply_journal_command_preview_reports_method_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("query"),
        Some("health"),
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "call".to_string(),
            "--query".to_string(),
            ROOT.to_string(),
            "health".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure fleet verification previews call the declared method on the target root.
#[test]
fn apply_journal_command_preview_reports_fleet_verification_command() {
    let journal = command_preview_journal(
        RestoreApplyOperationKind::VerifyFleet,
        Some("fleet-ready"),
        Some("canic_fleet_ready"),
    );
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "call".to_string(),
            "--query".to_string(),
            ROOT.to_string(),
            "canic_fleet_ready".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
    assert_eq!(
        command.note,
        "runs the declared fleet verification method as a query call"
    );
}

// Ensure method verification rows must carry the method they will call.
#[test]
fn apply_journal_validation_rejects_method_verification_without_method() {
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: None,
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::default(),
        pending_operations: 0,
        ready_operations: 1,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![RestoreApplyJournalOperation {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            state: RestoreApplyOperationState::Ready,
            state_updated_at: None,
            blocking_reasons: Vec::new(),
            restore_group: 1,
            phase_order: 0,
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            role: "root".to_string(),
            snapshot_id: Some("snap-root".to_string()),
            artifact_path: Some("artifacts/root".to_string()),
            verification_kind: Some("query".to_string()),
            verification_method: None,
        }],
        operation_receipts: Vec::new(),
    };

    let err = journal
        .validate()
        .expect_err("method verification without method should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            field: "operations[].verification_method",
        }
    ));
}

// Ensure apply journal validation rejects inconsistent state counts.
#[test]
fn apply_journal_validation_rejects_count_mismatch() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.blocked_operations = 0;

    let err = journal.validate().expect_err("count mismatch should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::CountMismatch {
            field: "blocked_operations",
            ..
        }
    ));
}

// Ensure supplied operation-kind counts must match concrete journal rows.
#[test]
fn apply_journal_validation_rejects_operation_kind_count_mismatch() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    journal.operation_counts = RestoreApplyOperationKindCounts {
        snapshot_uploads: 0,
        snapshot_loads: 1,
        code_reinstalls: 0,
        member_verifications: 0,
        fleet_verifications: 0,
        verification_operations: 0,
    };

    let err = journal
        .validate()
        .expect_err("operation-kind count mismatch should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::CountMismatch {
            field: "operation_counts.snapshot_uploads",
            reported: 0,
            actual: 1,
        }
    ));
}

// Ensure older journals without operation-kind counts still validate.
#[test]
fn apply_journal_defaults_missing_operation_kind_counts() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    journal.operation_counts =
        RestoreApplyOperationKindCounts::from_operations(&journal.operations);
    let mut value = serde_json::to_value(&journal).expect("serialize journal");
    value
        .as_object_mut()
        .expect("journal should serialize as an object")
        .remove("operation_counts");

    let decoded: RestoreApplyJournal =
        serde_json::from_value(value).expect("decode old journal shape");
    decoded.validate().expect("old journal should validate");
    let status = decoded.status();

    assert_eq!(
        decoded.operation_counts,
        RestoreApplyOperationKindCounts::default()
    );
    assert_eq!(status.operation_counts.snapshot_uploads, 1);
    assert_eq!(status.operation_counts.snapshot_loads, 0);
    assert!(!status.operation_counts_supplied);
}

// Ensure apply journal validation rejects duplicate operation sequences.
#[test]
fn apply_journal_validation_rejects_duplicate_sequences() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.operations[1].sequence = journal.operations[0].sequence;

    let err = journal
        .validate()
        .expect_err("duplicate sequence should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::DuplicateSequence(0)
    ));
}

// Ensure failed journal operations must explain why execution failed.
#[test]
fn apply_journal_validation_rejects_failed_without_reason() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal.operations[0].state = RestoreApplyOperationState::Failed;
    journal.operations[0].blocking_reasons = Vec::new();
    journal.blocked_operations -= 1;
    journal.failed_operations = 1;

    let err = journal
        .validate()
        .expect_err("failed operation without reason should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::FailureReasonRequired(0)
    ));
}

// Ensure claiming a ready operation marks it pending and keeps it resumable.
#[test]
fn apply_journal_mark_next_operation_pending_claims_first_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    let status = journal.status();
    let report = journal.report();
    let next = journal.next_operation();
    let preview = journal.next_command_preview();

    assert_eq!(journal.pending_operations, 1);
    assert_eq!(journal.ready_operations, 0);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Pending
    );
    assert_eq!(
        journal.operations[0].state_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert_eq!(status.next_ready_sequence, None);
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Pending)
    );
    assert_eq!(
        status.next_transition_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert_eq!(status.pending_summary.pending_operations, 1);
    assert!(status.pending_summary.pending_operation_available);
    assert_eq!(status.pending_summary.pending_sequence, Some(0));
    assert_eq!(
        status.pending_summary.pending_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
    assert_eq!(
        status.pending_summary.pending_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert!(status.pending_summary.pending_updated_at_known);
    assert_eq!(report.pending_summary, status.pending_summary);
    assert!(next.operation_available);
    assert_eq!(
        next.operation.expect("next operation").state,
        RestoreApplyOperationState::Pending
    );
    assert!(preview.operation_available);
    assert!(preview.command_available);
    assert_eq!(
        preview.operation.expect("preview operation").state,
        RestoreApplyOperationState::Pending
    );
}

// Ensure a pending claim can be released back to ready for retry.
#[test]
fn apply_journal_mark_next_operation_ready_unclaims_pending_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    journal
        .mark_next_operation_ready_at(Some("2026-05-04T12:01:00Z".to_string()))
        .expect("mark operation ready");
    let status = journal.status();
    let next = journal.next_operation();

    assert_eq!(journal.pending_operations, 0);
    assert_eq!(journal.ready_operations, 1);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert_eq!(
        journal.operations[0].state_updated_at.as_deref(),
        Some("2026-05-04T12:01:00Z")
    );
    assert_eq!(status.next_ready_sequence, Some(0));
    assert_eq!(status.next_transition_sequence, Some(0));
    assert_eq!(
        status.next_transition_state,
        Some(RestoreApplyOperationState::Ready)
    );
    assert_eq!(
        status.next_transition_updated_at.as_deref(),
        Some("2026-05-04T12:01:00Z")
    );
    assert_eq!(
        next.operation.expect("next operation").state,
        RestoreApplyOperationState::Ready
    );
}

// Ensure empty state update markers are rejected during journal validation.
#[test]
fn apply_journal_validation_rejects_empty_state_updated_at() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    journal.operations[0].state_updated_at = Some(String::new());
    let err = journal
        .validate()
        .expect_err("empty state update marker should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::MissingField("operations[].state_updated_at")
    ));
}

// Ensure operation-specific fields are required before command rendering.
#[test]
fn apply_journal_validation_rejects_missing_operation_fields() {
    let mut upload = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);
    upload.operations[0].artifact_path = None;
    let err = upload
        .validate()
        .expect_err("upload without artifact path should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::UploadSnapshot,
            field: "operations[].artifact_path",
        }
    ));

    let mut load = command_preview_journal(RestoreApplyOperationKind::LoadSnapshot, None, None);
    load.operations[0].snapshot_id = None;
    let err = load
        .validate()
        .expect_err("load without snapshot id should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::LoadSnapshot,
            field: "operations[].snapshot_id",
        }
    ));

    let mut verify = command_preview_journal(
        RestoreApplyOperationKind::VerifyMember,
        Some("query"),
        Some("health"),
    );
    verify.operations[0].verification_method = None;
    let err = verify
        .validate()
        .expect_err("method verification without method should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            field: "operations[].verification_method",
        }
    ));
}

// Ensure unclaim fails when the next transitionable operation is not pending.
#[test]
fn apply_journal_mark_next_operation_ready_rejects_without_pending_operation() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

    let err = journal
        .mark_next_operation_ready()
        .expect_err("ready operation should not unclaim");

    assert!(matches!(err, RestoreApplyJournalError::NoPendingOperation));
    assert_eq!(journal.ready_operations, 1);
    assert_eq!(journal.pending_operations, 0);
}

// Ensure pending claims cannot skip earlier ready operations.
#[test]
fn apply_journal_mark_pending_rejects_out_of_order_operation() {
    let root = temp_dir("canic-restore-apply-journal-pending-out-of-order");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_pending(1)
        .expect_err("out-of-order pending claim should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: 1,
            next: 0
        }
    ));
    assert_eq!(journal.pending_operations, 0);
    assert_eq!(journal.ready_operations, 6);
}

// Ensure completing a journal operation updates counts and advances status.
#[test]
fn apply_journal_mark_completed_advances_next_ready_operation() {
    let root = temp_dir("canic-restore-apply-journal-completed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_completed(0)
        .expect("mark operation completed");
    let status = journal.status();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(journal.completed_operations, 1);
    assert_eq!(journal.ready_operations, 5);
    assert_eq!(status.next_ready_sequence, Some(1));
    assert_eq!(status.progress.completed_operations, 1);
    assert_eq!(status.progress.remaining_operations, 5);
    assert_eq!(status.progress.transitionable_operations, 5);
    assert_eq!(status.progress.attention_operations, 0);
    assert_eq!(status.progress.completion_basis_points, 1666);
}

// Ensure journal transitions cannot skip earlier ready operations.
#[test]
fn apply_journal_mark_completed_rejects_out_of_order_operation() {
    let root = temp_dir("canic-restore-apply-journal-out-of-order");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed(1)
        .expect_err("out-of-order operation should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: 1,
            next: 0
        }
    ));
    assert_eq!(journal.completed_operations, 0);
    assert_eq!(journal.ready_operations, 6);
}

// Ensure failed journal operations carry a reason and update counts.
#[test]
fn apply_journal_mark_failed_records_reason() {
    let root = temp_dir("canic-restore-apply-journal-failed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_failed(0, "dfx-load-failed".to_string())
        .expect("mark operation failed");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Failed
    );
    assert_eq!(
        journal.operations[0].blocking_reasons,
        vec!["dfx-load-failed".to_string()]
    );
    assert_eq!(journal.failed_operations, 1);
    assert_eq!(journal.ready_operations, 5);
}

// Ensure failed operations can move back to ready for a retry.
#[test]
fn apply_journal_retry_failed_operation_marks_ready() {
    let root = temp_dir("canic-restore-apply-journal-retry-failed");
    fs::create_dir_all(&root).expect("create temp root");
    let mut manifest = valid_manifest(IdentityMode::Relocatable);
    set_member_artifact(
        &mut manifest,
        CHILD,
        &root,
        "artifacts/child",
        b"child-snapshot",
    );
    set_member_artifact(
        &mut manifest,
        ROOT,
        &root,
        "artifacts/root",
        b"root-snapshot",
    );

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_failed(0, "dfx-upload-failed".to_string())
        .expect("mark failed operation");
    journal
        .retry_failed_operation_at(0, Some("2026-05-04T12:03:00Z".to_string()))
        .expect("retry failed operation");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.failed_operations, 0);
    assert_eq!(journal.ready_operations, 6);
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(journal.operations[0].blocking_reasons.is_empty());
}

// Ensure blocked operations cannot be manually completed before blockers clear.
#[test]
fn apply_journal_rejects_blocked_operation_completion() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed(0)
        .expect_err("blocked operation should not complete");

    assert!(matches!(
        err,
        RestoreApplyJournalError::InvalidOperationTransition { sequence: 0, .. }
    ));
}
