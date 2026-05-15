use super::*;

// Ensure command output receipts keep bounded tail output and byte counts.
#[test]
fn apply_command_output_bounds_to_tail_bytes() {
    let output = RestoreApplyCommandOutput::from_bytes(b"abcdef", 3);

    assert_eq!(output.text, "def");
    assert!(output.truncated);
    assert_eq!(output.original_bytes, 6);
}

// Ensure hand-edited journals cannot duplicate one operation attempt outcome.
#[test]
fn apply_journal_rejects_duplicate_operation_receipt_attempts() {
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);
    journal
        .mark_operation_completed_at(0, None)
        .expect("mark upload completed");
    let receipt = RestoreApplyOperationReceipt::command_completed(
        &journal.operations[0],
        RestoreApplyRunnerCommand {
            program: "icp".to_string(),
            args: vec![
                "canister".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
                ROOT.to_string(),
            ],
            mutates: true,
            requires_stopped_canister: false,
            note: "Upload snapshot artifact to target canister".to_string(),
        },
        "exit:0".to_string(),
        Some("unix:1".to_string()),
        RestoreApplyCommandOutputPair::from_bytes(
            b"Uploaded snapshot: target-snap-root\n",
            b"",
            1024,
        ),
        1,
        Some("target-snap-root".to_string()),
    );
    journal
        .record_operation_receipt(receipt.clone())
        .expect("record first receipt");
    journal.operation_receipts.push(receipt);

    let err = journal
        .validate()
        .expect_err("duplicate receipt attempt should reject");

    assert!(matches!(
        err,
        RestoreApplyJournalError::DuplicateOperationReceiptAttempt {
            sequence: 0,
            attempt: 1,
        }
    ));
}

// Ensure command receipts preserve the durable command/output audit envelope.
#[test]
fn apply_journal_command_receipts_require_audit_fields() {
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);
    journal
        .mark_operation_completed_at(0, None)
        .expect("mark upload completed");
    let receipt = RestoreApplyOperationReceipt::command_completed(
        &journal.operations[0],
        RestoreApplyRunnerCommand {
            program: "icp".to_string(),
            args: vec![
                "canister".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
                ROOT.to_string(),
            ],
            mutates: true,
            requires_stopped_canister: false,
            note: "Upload snapshot artifact to target canister".to_string(),
        },
        "exit:0".to_string(),
        Some("unix:1".to_string()),
        RestoreApplyCommandOutputPair::from_bytes(
            b"Uploaded snapshot: target-snap-root\n",
            b"",
            1024,
        ),
        1,
        Some("target-snap-root".to_string()),
    );

    let mut missing_updated_at = receipt.clone();
    missing_updated_at.updated_at = None;
    assert_receipt_missing_field(
        &mut journal,
        missing_updated_at,
        "operation_receipts[].updated_at",
    );

    let mut missing_command = receipt.clone();
    missing_command.command = None;
    assert_receipt_missing_field(
        &mut journal,
        missing_command,
        "operation_receipts[].command",
    );

    let mut missing_status = receipt.clone();
    missing_status.status = None;
    assert_receipt_missing_field(&mut journal, missing_status, "operation_receipts[].status");

    let mut missing_stdout = receipt.clone();
    missing_stdout.stdout = None;
    assert_receipt_missing_field(&mut journal, missing_stdout, "operation_receipts[].stdout");

    let mut missing_stderr = receipt.clone();
    missing_stderr.stderr = None;
    assert_receipt_missing_field(&mut journal, missing_stderr, "operation_receipts[].stderr");

    let mut empty_program = receipt.clone();
    empty_program
        .command
        .as_mut()
        .expect("command")
        .program
        .clear();
    assert_receipt_missing_field(
        &mut journal,
        empty_program,
        "operation_receipts[].command.program",
    );

    let mut empty_args = receipt.clone();
    empty_args.command.as_mut().expect("command").args.clear();
    assert_receipt_missing_field(
        &mut journal,
        empty_args,
        "operation_receipts[].command.args",
    );

    let mut empty_note = receipt;
    empty_note.command.as_mut().expect("command").note.clear();
    assert_receipt_missing_field(
        &mut journal,
        empty_note,
        "operation_receipts[].command.note",
    );
}

fn assert_receipt_missing_field(
    journal: &mut RestoreApplyJournal,
    receipt: RestoreApplyOperationReceipt,
    field: &'static str,
) {
    let receipt_count = journal.operation_receipts.len();
    let err = journal
        .record_operation_receipt(receipt)
        .expect_err("receipt field should be required");

    assert!(matches!(
        err,
        RestoreApplyJournalError::MissingField(missing) if missing == field
    ));
    assert_eq!(journal.operation_receipts.len(), receipt_count);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.journal_version, 1);
    assert_eq!(journal.backup_id.as_str(), "fbk_test_001");
    assert!(journal.ready);
    assert!(journal.blocked_reasons.is_empty());
    assert_eq!(journal.operation_count, 10);
    assert_eq!(journal.ready_operations, 10);
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
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);

    assert!(!journal.ready);
    assert_eq!(journal.ready_operations, 0);
    assert_eq!(journal.blocked_operations, 10);
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

// Ensure apply journal report exposes progress, counters, and next transition.
#[test]
fn apply_journal_report_exposes_progress_and_next_transition() {
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let report = journal.report();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.report_version, 1);
    assert_eq!(report.backup_id.as_str(), "fbk_test_001");
    assert!(report.ready);
    assert!(!report.complete);
    assert_eq!(report.operation_count, 10);
    assert_eq!(report.operation_counts.canister_stops, 2);
    assert_eq!(report.operation_counts.canister_starts, 2);
    assert_eq!(report.operation_counts.snapshot_uploads, 2);
    assert_eq!(report.operation_counts.snapshot_loads, 2);
    assert_eq!(report.operation_counts.member_verifications, 2);
    assert_eq!(report.operation_counts.fleet_verifications, 0);
    assert_eq!(report.operation_counts.verification_operations, 2);
    assert_eq!(journal.operation_counts, report.operation_counts);
    assert_eq!(report.progress.operation_count, 10);
    assert_eq!(report.progress.completed_operations, 0);
    assert_eq!(report.progress.remaining_operations, 10);
    assert_eq!(report.progress.transitionable_operations, 10);
    assert_eq!(report.progress.attention_operations, 0);
    assert_eq!(report.progress.completion_basis_points, 0);
    assert_eq!(report.pending_summary.pending_operations, 0);
    assert!(!report.pending_summary.pending_operation_available);
    assert_eq!(report.pending_summary.pending_sequence, None);
    assert_eq!(report.pending_summary.pending_operation, None);
    assert_eq!(report.pending_summary.pending_updated_at, None);
    assert!(!report.pending_summary.pending_updated_at_known);
    assert_eq!(report.ready_operations, 10);
    let transition = report.next_transition.expect("next transition");
    assert_eq!(transition.sequence, 0);
    assert_eq!(transition.state, RestoreApplyOperationState::Ready);
    assert_eq!(
        transition.operation,
        RestoreApplyOperationKind::UploadSnapshot
    );
}

// Ensure command preview exposes the full next ready journal row.
#[test]
fn apply_journal_command_preview_reports_full_ready_row() {
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed_at(0, None)
        .expect("mark operation completed");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.ready);
    assert!(!preview.complete);
    assert!(preview.operation_available);
    let operation = preview.operation.expect("next operation");
    assert_eq!(operation.sequence, 1);
    assert_eq!(operation.state, RestoreApplyOperationState::Ready);
    assert_eq!(
        operation.operation,
        RestoreApplyOperationKind::UploadSnapshot
    );
    assert_eq!(operation.source_canister, CHILD);
}

// Ensure blocked journals report no preview operation.
#[test]
fn apply_journal_command_preview_reports_blocked_state() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview();

    assert!(!preview.ready);
    assert!(!preview.operation_available);
    assert!(preview.operation.is_none());
    assert!(
        preview
            .blocked_reasons
            .contains(&"missing-artifact-validation".to_string())
    );
}

// Ensure command previews expose the ICP CLI upload command without executing it.
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview();
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(preview.ready);
    assert!(preview.operation_available);
    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "icp");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            ROOT.to_string(),
            "--input".to_string(),
            expected_artifact_path,
            "--resume".to_string(),
            "--json".to_string(),
        ]
    );
    assert!(command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure command previews carry configured ICP CLI program and network.
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
        program: "/tmp/icp".to_string(),
        network: Some("local".to_string()),
    });
    let expected_artifact_path = root.join("artifacts/root").to_string_lossy().to_string();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(command.program, "/tmp/icp");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "-n".to_string(),
            "local".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            ROOT.to_string(),
            "--input".to_string(),
            expected_artifact_path,
            "--resume".to_string(),
            "--json".to_string(),
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed_at(0, None)
        .expect("mark root upload completed");
    journal
        .record_operation_receipt(RestoreApplyOperationReceipt::command_completed(
            &journal.operations[0],
            RestoreApplyRunnerCommand {
                program: "icp".to_string(),
                args: vec![
                    "canister".to_string(),
                    "snapshot".to_string(),
                    "upload".to_string(),
                    ROOT.to_string(),
                    "artifacts/root".to_string(),
                ],
                mutates: true,
                requires_stopped_canister: false,
                note: "Upload snapshot artifact to target canister".to_string(),
            },
            "exit:0".to_string(),
            Some("unix:1".to_string()),
            RestoreApplyCommandOutputPair::from_bytes(b"target-snap-root\n", b"", 1024),
            1,
            Some("target-snap-root".to_string()),
        ))
        .expect("record root upload receipt");
    journal
        .mark_operation_completed_at(1, None)
        .expect("mark child upload completed");
    journal
        .record_operation_receipt(RestoreApplyOperationReceipt::command_completed(
            &journal.operations[1],
            RestoreApplyRunnerCommand {
                program: "icp".to_string(),
                args: vec![
                    "canister".to_string(),
                    "snapshot".to_string(),
                    "upload".to_string(),
                    CHILD.to_string(),
                    "artifacts/child".to_string(),
                ],
                mutates: true,
                requires_stopped_canister: false,
                note: "Upload snapshot artifact to target canister".to_string(),
            },
            "exit:0".to_string(),
            Some("unix:1".to_string()),
            RestoreApplyCommandOutputPair::from_bytes(b"target-snap-child\n", b"", 1024),
            1,
            Some("target-snap-child".to_string()),
        ))
        .expect("record child upload receipt");
    journal
        .mark_operation_completed_at(2, None)
        .expect("mark root stop completed");
    journal
        .mark_operation_completed_at(3, None)
        .expect("mark child stop completed");
    let preview = journal.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "restore".to_string(),
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_completed_at(0, None)
        .expect("mark root upload completed");
    journal
        .mark_operation_completed_at(1, None)
        .expect("mark child upload completed");
    journal
        .record_operation_receipt(RestoreApplyOperationReceipt::command_completed(
            &journal.operations[1],
            RestoreApplyRunnerCommand {
                program: "icp".to_string(),
                args: vec![
                    "canister".to_string(),
                    "snapshot".to_string(),
                    "upload".to_string(),
                    CHILD.to_string(),
                    "artifacts/child".to_string(),
                ],
                mutates: true,
                requires_stopped_canister: false,
                note: "Upload snapshot artifact to target canister".to_string(),
            },
            "exit:0".to_string(),
            Some("unix:1".to_string()),
            RestoreApplyCommandOutputPair::from_bytes(b"target-snap-child\n", b"", 1024),
            1,
            Some("target-snap-child".to_string()),
        ))
        .expect("record child upload receipt");
    journal
        .mark_operation_completed_at(2, None)
        .expect("mark root stop completed");
    journal
        .mark_operation_completed_at(3, None)
        .expect("mark child stop completed");
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

// Ensure status verification previews use `icp canister status`.
#[test]
fn apply_journal_command_preview_reports_status_verification_command() {
    let journal = command_preview_journal(RestoreApplyOperationKind::VerifyMember, Some("status"));
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "status".to_string(),
            ROOT.to_string(),
            "--json".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
}

// Ensure unsupported verification kinds do not render runner commands.
#[test]
fn apply_journal_command_preview_rejects_unsupported_verification_command() {
    let mut journal =
        command_preview_journal(RestoreApplyOperationKind::VerifyMember, Some("status"));
    journal.operations[0].verification_kind = Some("query".to_string());
    let preview = journal.next_command_preview();

    assert!(!preview.command_available);
    assert!(preview.command.is_none());
}

// Ensure fleet verification previews check target root status.
#[test]
fn apply_journal_command_preview_reports_fleet_verification_command() {
    let journal = command_preview_journal(RestoreApplyOperationKind::VerifyFleet, Some("status"));
    let preview = journal.next_command_preview();

    assert!(preview.command_available);
    let command = preview.command.expect("command preview");
    assert_eq!(
        command.args,
        vec![
            "canister".to_string(),
            "status".to_string(),
            ROOT.to_string(),
            "--json".to_string(),
        ]
    );
    assert!(!command.mutates);
    assert!(!command.requires_stopped_canister);
    assert_eq!(command.note, "checks target fleet root canister status");
}

// Ensure unsupported verification rows are rejected before execution.
#[test]
fn apply_journal_validation_rejects_unsupported_verification_kind() {
    let mut journal = RestoreApplyJournal {
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
            member_order: 0,
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            role: "root".to_string(),
            snapshot_id: Some("snap-root".to_string()),
            artifact_path: Some("artifacts/root".to_string()),
            verification_kind: Some("query".to_string()),
        }],
        operation_receipts: Vec::new(),
    };
    journal.operation_counts =
        RestoreApplyOperationKindCounts::from_operations(&journal.operations);

    let err = journal
        .validate()
        .expect_err("unsupported verification kind should fail");

    assert!(matches!(
        err,
        RestoreApplyJournalError::UnsupportedVerificationKind { sequence: 0, .. }
    ));
}

// Ensure apply journal validation rejects inconsistent state counts.
#[test]
fn apply_journal_validation_rejects_count_mismatch() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
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
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);
    journal.operation_counts = RestoreApplyOperationKindCounts {
        canister_stops: 0,
        canister_starts: 0,
        snapshot_uploads: 0,
        snapshot_loads: 1,
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

// Ensure apply journal validation rejects duplicate operation sequences.
#[test]
fn apply_journal_validation_rejects_duplicate_sequences() {
    let manifest = valid_manifest(IdentityMode::Relocatable);

    let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
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
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
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
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    let report = journal.report();
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
    assert!(report.next_transition.is_some());
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .map(|operation| &operation.state),
        Some(&RestoreApplyOperationState::Pending)
    );
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .and_then(|operation| operation.state_updated_at.as_deref()),
        Some("2026-05-04T12:00:00Z")
    );
    assert_eq!(report.pending_summary.pending_operations, 1);
    assert!(report.pending_summary.pending_operation_available);
    assert_eq!(report.pending_summary.pending_sequence, Some(0));
    assert_eq!(
        report.pending_summary.pending_operation,
        Some(RestoreApplyOperationKind::UploadSnapshot)
    );
    assert_eq!(
        report.pending_summary.pending_updated_at.as_deref(),
        Some("2026-05-04T12:00:00Z")
    );
    assert!(report.pending_summary.pending_updated_at_known);
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
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);

    journal
        .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
        .expect("mark operation pending");
    journal
        .mark_next_operation_ready_at(Some("2026-05-04T12:01:00Z".to_string()))
        .expect("mark operation ready");
    let report = journal.report();
    let preview = journal.next_command_preview();

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
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .map(|operation| operation.sequence),
        Some(0)
    );
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .map(|operation| &operation.state),
        Some(&RestoreApplyOperationState::Ready)
    );
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .and_then(|operation| operation.state_updated_at.as_deref()),
        Some("2026-05-04T12:01:00Z")
    );
    assert_eq!(
        preview.operation.expect("next operation").state,
        RestoreApplyOperationState::Ready
    );
}

// Ensure empty state update markers are rejected during journal validation.
#[test]
fn apply_journal_validation_rejects_empty_state_updated_at() {
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);

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
    let mut upload = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);
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

    let mut load = command_preview_journal(RestoreApplyOperationKind::LoadSnapshot, None);
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

    let mut verify =
        command_preview_journal(RestoreApplyOperationKind::VerifyMember, Some("status"));
    verify.operations[0].verification_kind = None;
    let err = verify
        .validate()
        .expect_err("missing verification kind should fail");
    assert!(matches!(
        err,
        RestoreApplyJournalError::OperationMissingField {
            sequence: 0,
            operation: RestoreApplyOperationKind::VerifyMember,
            field: "operations[].verification_kind",
        }
    ));
}

// Ensure unclaim fails when the next transitionable operation is not pending.
#[test]
fn apply_journal_mark_next_operation_ready_rejects_without_pending_operation() {
    let mut journal = command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None);

    let err = journal
        .mark_next_operation_ready_at(None)
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_pending_at(1, None)
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
    assert_eq!(journal.ready_operations, 10);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_completed_at(0, None)
        .expect("mark operation completed");
    let report = journal.report();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(journal.completed_operations, 1);
    assert_eq!(journal.ready_operations, 9);
    assert_eq!(
        report
            .next_transition
            .as_ref()
            .map(|operation| operation.sequence),
        Some(1)
    );
    assert_eq!(report.progress.completed_operations, 1);
    assert_eq!(report.progress.remaining_operations, 9);
    assert_eq!(report.progress.transitionable_operations, 9);
    assert_eq!(report.progress.attention_operations, 0);
    assert_eq!(report.progress.completion_basis_points, 1000);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed_at(1, None)
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
    assert_eq!(journal.ready_operations, 10);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal
        .mark_operation_failed_at(0, "icp-load-failed".to_string(), None)
        .expect("mark operation failed");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(
        journal.operations[0].state,
        RestoreApplyOperationState::Failed
    );
    assert_eq!(
        journal.operations[0].blocking_reasons,
        vec!["icp-load-failed".to_string()]
    );
    assert_eq!(journal.failed_operations, 1);
    assert_eq!(journal.ready_operations, 9);
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
    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &root)
        .expect("dry-run should validate artifacts");
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
    journal
        .mark_operation_failed_at(0, "icp-upload-failed".to_string(), None)
        .expect("mark failed operation");
    journal
        .retry_failed_operation_at(0, Some("2026-05-04T12:03:00Z".to_string()))
        .expect("retry failed operation");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(journal.failed_operations, 0);
    assert_eq!(journal.ready_operations, 10);
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
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    let err = journal
        .mark_operation_completed_at(0, None)
        .expect_err("blocked operation should not complete");

    assert!(matches!(
        err,
        RestoreApplyJournalError::InvalidOperationTransition { sequence: 0, .. }
    ));
}
