use super::*;
use crate::test_support::temp_dir;
use canic_backup::restore::{RestoreApplyJournal, RestoreApplyOperationState};
use serde_json::json;
use std::{ffi::OsString, fs};

// Ensure restore run writes a native no-mutation runner preview.
#[test]
fn run_restore_run_dry_run_writes_native_runner_preview() {
    let root = temp_dir("canic-cli-restore-run-dry-run");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run-dry-run.json");
    let journal = ready_apply_journal();

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--dry-run"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("write restore run dry-run");

    let dry_run: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
            .expect("decode dry-run");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(dry_run["run_version"], 1);
    assert_eq!(dry_run["backup_id"], "backup-test");
    assert_eq!(dry_run["run_mode"], "dry-run");
    assert_eq!(dry_run["dry_run"], true);
    assert_eq!(
        dry_run["requested_state_updated_at"],
        serde_json::Value::Null
    );
    assert_eq!(dry_run["ready"], true);
    assert_eq!(dry_run["complete"], false);
    assert_eq!(dry_run["attention_required"], false);
    assert_eq!(dry_run["operation_counts"]["canister_stops"], 2);
    assert_eq!(dry_run["operation_counts"]["canister_starts"], 2);
    assert_eq!(dry_run["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(dry_run["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(dry_run["operation_counts"]["member_verifications"], 2);
    assert_eq!(dry_run["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(dry_run["operation_counts"]["verification_operations"], 2);
    assert_eq!(dry_run["progress"]["operation_count"], 10);
    assert_eq!(dry_run["progress"]["completed_operations"], 0);
    assert_eq!(dry_run["progress"]["remaining_operations"], 10);
    assert_eq!(dry_run["progress"]["transitionable_operations"], 10);
    assert_eq!(dry_run["progress"]["attention_operations"], 0);
    assert_eq!(dry_run["progress"]["completion_basis_points"], 0);
    assert_eq!(dry_run["pending_summary"]["pending_operations"], 0);
    assert_eq!(
        dry_run["pending_summary"]["pending_operation_available"],
        false
    );
    assert_eq!(dry_run["operation_receipt_count"], 0);
    assert_eq!(dry_run["operation_receipt_summary"]["total_receipts"], 0);
    assert_eq!(dry_run["operation_receipt_summary"]["command_completed"], 0);
    assert_eq!(dry_run["operation_receipt_summary"]["command_failed"], 0);
    assert_eq!(dry_run["operation_receipt_summary"]["pending_recovered"], 0);
    assert!(dry_run.get("batch_summary").is_none());
    assert_eq!(dry_run["stopped_reason"], "preview");
    assert_eq!(dry_run["next_action"], "rerun");
    assert_eq!(dry_run["operation_available"], true);
    assert_eq!(dry_run["command_available"], true);
    assert_eq!(dry_run["next_transition"]["sequence"], 0);
    assert_eq!(dry_run["command"]["program"], "/tmp/icp");
    assert_eq!(
        dry_run["command"]["args"],
        json!([
            "canister",
            "-n",
            "local",
            "snapshot",
            "upload",
            ROOT,
            "--input",
            "/tmp/canic-cli-restore-artifacts/artifacts/root",
            "--resume",
            "--json"
        ])
    );
    assert_eq!(dry_run["command"]["mutates"], true);
}

// Ensure restore run can recover one interrupted pending operation.
#[test]
fn run_restore_run_unclaim_pending_marks_operation_ready() {
    let root = temp_dir("canic-cli-restore-run-unclaim-pending");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run.json");
    let mut journal = ready_apply_journal();
    journal
        .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
        .expect("mark pending operation");

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--unclaim-pending"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("unclaim pending operation");

    let run_summary: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
            .expect("decode run summary");
    let updated: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
            .expect("decode updated journal");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(run_summary["run_mode"], "unclaim-pending");
    assert_eq!(run_summary["unclaim_pending"], true);
    assert_eq!(run_summary["stopped_reason"], "recovered-pending");
    assert_eq!(run_summary["next_action"], "rerun");
    assert_eq!(
        run_summary["requested_state_updated_at"],
        serde_json::Value::Null
    );
    assert_eq!(run_summary["recovered_operation"]["sequence"], 0);
    assert_eq!(run_summary["recovered_operation"]["state"], "pending");
    assert_eq!(run_summary["operation_receipt_count"], 1);
    assert_eq!(
        run_summary["operation_receipt_summary"]["total_receipts"],
        1
    );
    assert!(run_summary.get("batch_summary").is_none());
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_completed"],
        0
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_failed"],
        0
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["pending_recovered"],
        1
    );
    assert_eq!(
        run_summary["operation_receipts"][0]["event"],
        "pending-recovered"
    );
    assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
    assert_eq!(run_summary["operation_receipts"][0]["state"], "ready");
    assert!(
        run_summary["operation_receipts"][0]["updated_at"]
            .as_str()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
    assert_eq!(run_summary["pending_operations"], 0);
    assert_eq!(run_summary["ready_operations"], 10);
    assert_eq!(run_summary["attention_required"], false);
    assert_eq!(updated.pending_operations, 0);
    assert_eq!(updated.ready_operations, 10);
    assert_eq!(
        updated.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(
        updated.operations[0]
            .state_updated_at
            .as_deref()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
}

// Ensure restore run execute claims and completes one generated command.
#[test]
fn run_restore_run_execute_marks_completed_operation() {
    let root = temp_dir("canic-cli-restore-run-execute");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run.json");
    let journal = ready_apply_journal();

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--execute"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/true"),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("execute one restore run step");

    let run_summary: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
            .expect("decode run summary");
    let updated: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
            .expect("decode updated journal");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(run_summary["run_mode"], "execute");
    assert_eq!(run_summary["execute"], true);
    assert_eq!(run_summary["dry_run"], false);
    assert_eq!(run_summary["max_steps_reached"], true);
    assert_eq!(run_summary["stopped_reason"], "max-steps-reached");
    assert_eq!(run_summary["next_action"], "rerun");
    assert_eq!(
        run_summary["requested_state_updated_at"],
        serde_json::Value::Null
    );
    assert_eq!(run_summary["executed_operation_count"], 1);
    assert!(run_summary.get("batch_summary").is_none());
    assert_eq!(run_summary["operation_receipt_count"], 1);
    assert_eq!(
        run_summary["operation_receipt_summary"]["total_receipts"],
        1
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_completed"],
        1
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_failed"],
        0
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["pending_recovered"],
        0
    );
    assert_eq!(run_summary["executed_operations"][0]["sequence"], 0);
    assert_eq!(
        run_summary["executed_operations"][0]["command"]["program"],
        "/bin/true"
    );
    assert_eq!(
        run_summary["operation_receipts"][0]["event"],
        "command-completed"
    );
    assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
    assert_eq!(run_summary["operation_receipts"][0]["state"], "completed");
    assert_eq!(
        run_summary["operation_receipts"][0]["command"]["program"],
        "/bin/true"
    );
    assert_eq!(run_summary["operation_receipts"][0]["status"], "0");
    assert!(
        run_summary["operation_receipts"][0]["updated_at"]
            .as_str()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
    assert_eq!(updated.completed_operations, 1);
    assert_eq!(updated.pending_operations, 0);
    assert_eq!(updated.failed_operations, 0);
    assert_eq!(
        updated.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert!(
        updated.operations[0]
            .state_updated_at
            .as_deref()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
}

// Ensure successful upload commands persist target snapshot IDs in the journal.
#[cfg(unix)]
#[test]
fn run_restore_run_execute_records_uploaded_snapshot_receipt() {
    let root = temp_dir("canic-cli-restore-run-upload-receipt");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run.json");
    let fake_icp = write_fake_icp_upload(&root, "target-snap-root");
    let journal = ready_apply_journal();

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--execute"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from(fake_icp.as_os_str()),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("execute upload step");

    let updated: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
            .expect("decode updated journal");
    let preview = updated.next_command_preview();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(updated.operation_receipts.len(), 1);
    assert_eq!(updated.operation_receipts[0].attempt, 1);
    assert_eq!(updated.operation_receipts[0].status.as_deref(), Some("0"));
    assert_eq!(
        updated.operation_receipts[0]
            .uploaded_snapshot_id
            .as_deref(),
        Some("target-snap-root")
    );
    assert_eq!(
        updated.operation_receipts[0]
            .stdout
            .as_ref()
            .map(|output| output.text.as_str()),
        Some("Uploaded snapshot: target-snap-root\n")
    );
    assert_eq!(
        preview.command.expect("next upload command").args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
            CHILD.to_string(),
            "--input".to_string(),
            "/tmp/canic-cli-restore-artifacts/artifacts/app".to_string(),
            "--resume".to_string(),
            "--json".to_string(),
        ]
    );
}

// Ensure native runner execution refuses a journal that is already locked.
#[test]
fn run_restore_run_execute_rejects_locked_journal() {
    let fixture =
        RestoreCliFixture::new("canic-cli-restore-run-locked-journal", "restore-run.json");
    let journal = ready_apply_journal();
    fixture.write_journal(&journal);
    let lock_path = journal_lock_path(&fixture.journal_path);
    fs::write(&lock_path, "pid=other\n").expect("write existing lock");

    let err = fixture
        .run_restore_run(&[
            "--execute",
            crate::cli::globals::INTERNAL_ICP_OPTION,
            "/bin/true",
            "--max-steps",
            "1",
        ])
        .expect_err("locked journal should reject execution");

    assert!(matches!(
        err,
        RestoreCommandError::RestoreApplyJournalLocked { .. }
    ));
    assert!(lock_path.exists());
}

// Ensure restore run can fail closed after writing an incomplete summary.
#[test]
fn run_restore_run_require_complete_writes_summary_then_fails() {
    let root = temp_dir("canic-cli-restore-run-require-complete");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run.json");
    let journal = ready_apply_journal();

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    let err = run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--execute"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/true"),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--require-complete"),
    ])
    .expect_err("incomplete run should fail requirement");

    let run_summary: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
            .expect("decode run summary");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(run_summary["executed_operation_count"], 1);
    assert_eq!(run_summary["complete"], false);
    assert!(matches!(
        err,
        RestoreCommandError::RestoreApplyIncomplete {
            completed_operations: 1,
            operation_count: 10,
            ..
        }
    ));
}

// Ensure restore run execute records failed command exits in the journal.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "failure-path fixture asserts persisted journal state and emitted summary shape"
)]
fn run_restore_run_execute_marks_failed_operation() {
    let root = temp_dir("canic-cli-restore-run-execute-failed");
    fs::create_dir_all(&root).expect("create temp root");
    let journal_path = root.join("restore-apply-journal.json");
    let out_path = root.join("restore-run.json");
    let journal = ready_apply_journal();

    fs::write(
        &journal_path,
        serde_json::to_vec(&journal).expect("serialize journal"),
    )
    .expect("write journal");

    let err = run([
        OsString::from("run"),
        OsString::from("--journal"),
        OsString::from(journal_path.as_os_str()),
        OsString::from("--execute"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/false"),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect_err("failing runner command should fail");

    let run_summary: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
            .expect("decode run summary");
    let updated: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
            .expect("decode updated journal");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        RestoreCommandError::RestoreRunCommandFailed {
            sequence: 0,
            status,
        } if status == "1"
    ));
    assert_eq!(updated.failed_operations, 1);
    assert_eq!(updated.pending_operations, 0);
    assert_eq!(
        updated.operations[0].state,
        RestoreApplyOperationState::Failed
    );
    assert_eq!(run_summary["execute"], true);
    assert_eq!(run_summary["attention_required"], true);
    assert_eq!(run_summary["outcome"], "failed");
    assert_eq!(run_summary["stopped_reason"], "command-failed");
    assert_eq!(run_summary["next_action"], "inspect-failed-operation");
    assert_eq!(
        run_summary["requested_state_updated_at"],
        serde_json::Value::Null
    );
    assert_eq!(run_summary["executed_operation_count"], 1);
    assert_eq!(run_summary["operation_receipt_count"], 1);
    assert_eq!(
        run_summary["operation_receipt_summary"]["total_receipts"],
        1
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_completed"],
        0
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["command_failed"],
        1
    );
    assert_eq!(
        run_summary["operation_receipt_summary"]["pending_recovered"],
        0
    );
    assert_eq!(run_summary["executed_operations"][0]["state"], "failed");
    assert_eq!(run_summary["executed_operations"][0]["status"], "1");
    assert_eq!(
        run_summary["operation_receipts"][0]["event"],
        "command-failed"
    );
    assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
    assert_eq!(run_summary["operation_receipts"][0]["state"], "failed");
    assert_eq!(
        run_summary["operation_receipts"][0]["command"]["program"],
        "/bin/false"
    );
    assert_eq!(run_summary["operation_receipts"][0]["status"], "1");
    assert!(
        run_summary["operation_receipts"][0]["updated_at"]
            .as_str()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
    assert_eq!(updated.operation_receipts.len(), 1);
    assert_eq!(updated.operation_receipts[0].attempt, 1);
    assert_eq!(
        updated.operation_receipts[0].failure_reason.as_deref(),
        Some("runner-command-exit-1")
    );
    assert_eq!(updated.operation_receipts[0].status.as_deref(), Some("1"));
    assert_eq!(
        updated.operation_receipts[0]
            .stderr
            .as_ref()
            .map(|output| output.original_bytes),
        Some(0)
    );
    assert!(
        updated.operations[0]
            .state_updated_at
            .as_deref()
            .is_some_and(|updated_at| updated_at.starts_with("unix:"))
    );
    assert_eq!(
        updated.operations[0].blocking_reasons,
        vec!["runner-command-exit-1".to_string()]
    );
}

// Ensure restore run can fail closed after writing an attention summary.
#[test]
fn run_restore_run_require_no_attention_writes_summary_then_fails() {
    let fixture = RestoreCliFixture::new(
        "canic-cli-restore-run-require-attention",
        "restore-run.json",
    );
    let mut journal = ready_apply_journal();
    journal
        .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
        .expect("mark pending operation");
    fixture.write_journal(&journal);

    let err = fixture
        .run_restore_run(&["--dry-run", "--require-no-attention"])
        .expect_err("attention run should fail requirement");

    let run_summary: serde_json::Value = fixture.read_out("read run summary");

    assert_eq!(run_summary["attention_required"], true);
    assert_eq!(run_summary["outcome"], "pending");
    assert_eq!(run_summary["stopped_reason"], "pending");
    assert_eq!(run_summary["next_action"], "unclaim-pending");
    assert_eq!(run_summary["pending_summary"]["pending_sequence"], 0);
    assert_eq!(
        run_summary["pending_summary"]["pending_updated_at"],
        "2026-05-05T12:01:00Z"
    );
    assert!(matches!(
        err,
        RestoreCommandError::RestoreApplyReportNeedsAttention {
            outcome: canic_backup::restore::RestoreApplyReportOutcome::Pending,
            ..
        }
    ));
}
