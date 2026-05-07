use super::*;
use canic_backup::{
    artifacts::ArtifactChecksum,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
    persistence::BackupLayout,
    restore::{RestoreApplyJournal, RestoreApplyOperationState},
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const MAPPED_CHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

///
/// RestoreCliFixture
///

struct RestoreCliFixture {
    root: PathBuf,
    journal_path: PathBuf,
    out_path: PathBuf,
}

impl RestoreCliFixture {
    // Create a temp restore CLI fixture with canonical journal and output paths.
    fn new(prefix: &str, out_file: &str) -> Self {
        let root = temp_dir(prefix);
        fs::create_dir_all(&root).expect("create temp root");

        Self {
            journal_path: root.join("restore-apply-journal.json"),
            out_path: root.join(out_file),
            root,
        }
    }

    // Persist a restore apply journal at the fixture journal path.
    fn write_journal(&self, journal: &RestoreApplyJournal) {
        fs::write(
            &self.journal_path,
            serde_json::to_vec(journal).expect("serialize journal"),
        )
        .expect("write journal");
    }

    // Run restore-run against the fixture journal and output paths.
    fn run_restore_run(&self, extra: &[&str]) -> Result<(), RestoreCommandError> {
        self.run_journal_command("run", extra)
    }

    // Read the fixture output as a typed JSON value.
    fn read_out<T>(&self, label: &str) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(&fs::read(&self.out_path).expect(label)).expect(label)
    }

    // Build and run one journal-backed restore CLI command.
    fn run_journal_command(
        &self,
        command: &str,
        extra: &[&str],
    ) -> Result<(), RestoreCommandError> {
        let mut args = vec![
            OsString::from(command),
            OsString::from("--journal"),
            OsString::from(self.journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(self.out_path.as_os_str()),
        ];
        args.extend(extra.iter().map(OsString::from));
        run(args)
    }
}

impl Drop for RestoreCliFixture {
    // Remove the fixture directory after each test completes.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// Write a tiny fake dfx executable that reports one uploaded snapshot ID.
#[cfg(unix)]
fn write_fake_dfx_upload(root: &Path, uploaded_snapshot_id: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = root.join("dfx-upload-ok");
    fs::write(
        &path,
        format!("#!/bin/sh\nprintf 'Uploaded snapshot: {uploaded_snapshot_id}\\n'\n"),
    )
    .expect("write fake dfx");
    let mut permissions = fs::metadata(&path)
        .expect("fake dfx metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake dfx executable");
    path
}

// Ensure restore plan options parse the intended no-mutation command.
#[test]
fn parses_restore_plan_options() {
    let options = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--mapping"),
        OsString::from("mapping.json"),
        OsString::from("--out"),
        OsString::from("plan.json"),
        OsString::from("--require-restore-ready"),
    ])
    .expect("parse options");

    assert_eq!(options.manifest, Some(PathBuf::from("manifest.json")));
    assert_eq!(options.backup_dir, None);
    assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
    assert_eq!(options.out, Some(PathBuf::from("plan.json")));
    assert!(!options.require_verified);
    assert!(options.require_restore_ready);
}

// Ensure restore help stays at command-family level.
#[test]
fn restore_usage_lists_command_family() {
    let text = usage();

    assert!(text.contains("usage: canic restore <command> [<args>]"));
    assert!(text.contains("plan"));
    assert!(text.contains("run"));
}

// Ensure uploaded snapshot IDs are parsed from dfx upload output.
#[test]
fn parses_uploaded_snapshot_id_from_dfx_output() {
    let snapshot_id = parse_uploaded_snapshot_id("Uploaded snapshot: target-snap-001\n");

    assert_eq!(snapshot_id.as_deref(), Some("target-snap-001"));
}

// Ensure verified restore plan options parse with the canonical backup source.
#[test]
fn parses_verified_restore_plan_options() {
    let options = RestorePlanOptions::parse([
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
        OsString::from("--require-verified"),
    ])
    .expect("parse verified options");

    assert_eq!(options.manifest, None);
    assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
    assert_eq!(options.mapping, None);
    assert_eq!(options.out, None);
    assert!(options.require_verified);
    assert!(!options.require_restore_ready);
}

// Ensure restore apply options require the explicit dry-run mode.
#[test]
fn parses_restore_apply_dry_run_options() {
    let options = RestoreApplyOptions::parse([
        OsString::from("--plan"),
        OsString::from("restore-plan.json"),
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from("restore-apply-dry-run.json"),
        OsString::from("--journal-out"),
        OsString::from("restore-apply-journal.json"),
    ])
    .expect("parse apply options");

    assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
    assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
    assert_eq!(
        options.out,
        Some(PathBuf::from("restore-apply-dry-run.json"))
    );
    assert_eq!(
        options.journal_out,
        Some(PathBuf::from("restore-apply-journal.json"))
    );
    assert!(options.dry_run);
}

// Ensure restore run options parse the native runner dry-run command.
#[test]
fn parses_restore_run_dry_run_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--dry-run"),
        OsString::from("--dfx"),
        OsString::from("/tmp/dfx"),
        OsString::from("--network"),
        OsString::from("local"),
        OsString::from("--out"),
        OsString::from("restore-run-dry-run.json"),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--require-complete"),
        OsString::from("--require-no-attention"),
    ])
    .expect("parse restore run options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.dfx, "/tmp/dfx");
    assert_eq!(options.network.as_deref(), Some("local"));
    assert_eq!(options.out, Some(PathBuf::from("restore-run-dry-run.json")));
    assert!(options.dry_run);
    assert!(!options.execute);
    assert!(!options.unclaim_pending);
    assert_eq!(options.max_steps, Some(1));
    assert!(options.require_complete);
    assert!(options.require_no_attention);
}

// Ensure restore run options parse the native execute command.
#[test]
fn parses_restore_run_execute_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--execute"),
        OsString::from("--dfx"),
        OsString::from("/bin/true"),
        OsString::from("--max-steps"),
        OsString::from("4"),
    ])
    .expect("parse restore run execute options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.dfx, "/bin/true");
    assert_eq!(options.network, None);
    assert_eq!(options.out, None);
    assert!(!options.dry_run);
    assert!(options.execute);
    assert!(!options.unclaim_pending);
    assert_eq!(options.max_steps, Some(4));
    assert!(!options.require_complete);
    assert!(!options.require_no_attention);
}

// Ensure restore run options parse the native pending-operation recovery mode.
#[test]
fn parses_restore_run_unclaim_pending_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--unclaim-pending"),
        OsString::from("--out"),
        OsString::from("restore-run.json"),
    ])
    .expect("parse restore run unclaim options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.out, Some(PathBuf::from("restore-run.json")));
    assert!(!options.dry_run);
    assert!(!options.execute);
    assert!(options.unclaim_pending);
}

// Ensure restore apply only renders no-mutation operation plans.
#[test]
fn restore_apply_requires_dry_run() {
    let err = RestoreApplyOptions::parse([
        OsString::from("--plan"),
        OsString::from("restore-plan.json"),
    ])
    .expect_err("apply without dry-run should fail");

    assert!(matches!(err, RestoreCommandError::ApplyRequiresDryRun));
}

// Ensure restore run requires an explicit execution mode.
#[test]
fn restore_run_requires_mode() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
    ])
    .expect_err("restore run without dry-run should fail");

    assert!(matches!(err, RestoreCommandError::RestoreRunRequiresMode));
}

// Ensure restore run rejects ambiguous execution modes.
#[test]
fn restore_run_rejects_conflicting_modes() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--dry-run"),
        OsString::from("--execute"),
        OsString::from("--unclaim-pending"),
    ])
    .expect_err("restore run should reject conflicting modes");

    assert!(matches!(
        err,
        RestoreCommandError::RestoreRunConflictingModes
    ));
}

// Ensure restore run rejects zero-length execute batches.
#[test]
fn restore_run_rejects_zero_max_steps() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--execute"),
        OsString::from("--max-steps"),
        OsString::from("0"),
    ])
    .expect_err("restore run should reject zero max steps");

    assert!(matches!(
        err,
        RestoreCommandError::InvalidPositiveInteger {
            option: "--max-steps"
        }
    ));
}

// Ensure backup-dir restore planning reads the canonical layout manifest.
#[test]
fn plan_restore_reads_manifest_from_backup_dir() {
    let root = temp_dir("canic-cli-restore-plan-layout");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");

    let options = RestorePlanOptions {
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: false,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan restore");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(plan.backup_id, "backup-test");
    assert_eq!(plan.member_count, 2);
}

// Ensure restore planning has exactly one manifest source.
#[test]
fn parse_rejects_conflicting_manifest_sources() {
    let err = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
    ])
    .expect_err("conflicting sources should fail");

    assert!(matches!(
        err,
        RestoreCommandError::ConflictingManifestSources
    ));
}

// Ensure verified planning requires the canonical backup layout source.
#[test]
fn parse_rejects_require_verified_with_manifest_source() {
    let err = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--require-verified"),
    ])
    .expect_err("verification should require a backup layout");

    assert!(matches!(
        err,
        RestoreCommandError::RequireVerifiedNeedsBackupDir
    ));
}

// Ensure restore planning can require manifest, journal, and artifact integrity.
#[test]
fn plan_restore_requires_verified_backup_layout() {
    let root = temp_dir("canic-cli-restore-plan-verified");
    let layout = BackupLayout::new(root.clone());
    let manifest = valid_manifest();
    write_verified_layout(&root, &layout, &manifest);

    let options = RestorePlanOptions {
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: true,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan verified restore");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(plan.backup_id, "backup-test");
    assert_eq!(plan.member_count, 2);
}

// Ensure required verification fails before planning when the layout is incomplete.
#[test]
fn plan_restore_rejects_unverified_backup_layout() {
    let root = temp_dir("canic-cli-restore-plan-unverified");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");

    let options = RestorePlanOptions {
        manifest: None,
        backup_dir: Some(root.clone()),
        mapping: None,
        out: None,
        require_verified: true,
        require_restore_ready: false,
    };

    let err = plan_restore(&options).expect_err("missing journal should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(err, RestoreCommandError::Persistence(_)));
}

// Ensure the CLI planning path validates manifests and applies mappings.
#[test]
fn plan_restore_reads_manifest_and_mapping() {
    let root = temp_dir("canic-cli-restore-plan");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let mapping_path = root.join("mapping.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");
    fs::write(
        &mapping_path,
        json!({
            "members": [
                {"source_canister": ROOT, "target_canister": ROOT},
                {"source_canister": CHILD, "target_canister": MAPPED_CHILD}
            ]
        })
        .to_string(),
    )
    .expect("write mapping");

    let options = RestorePlanOptions {
        manifest: Some(manifest_path),
        backup_dir: None,
        mapping: Some(mapping_path),
        out: None,
        require_verified: false,
        require_restore_ready: false,
    };

    let plan = plan_restore(&options).expect("plan restore");

    fs::remove_dir_all(root).expect("remove temp root");
    let members = plan.ordered_members();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].source_canister, ROOT);
    assert_eq!(members[1].target_canister, MAPPED_CHILD);
}

// Ensure restore-readiness gating happens after writing the plan artifact.
#[test]
fn run_restore_plan_require_restore_ready_writes_plan_then_fails() {
    let root = temp_dir("canic-cli-restore-plan-require-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let out_path = root.join("plan.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");

    let err = run([
        OsString::from("plan"),
        OsString::from("--manifest"),
        OsString::from(manifest_path.as_os_str()),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--require-restore-ready"),
    ])
    .expect_err("restore readiness should be enforced");

    assert!(out_path.exists());
    let plan: RestorePlan =
        serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(!plan.readiness_summary.ready);
    assert!(matches!(
        err,
        RestoreCommandError::RestoreNotReady {
            reasons,
            ..
        } if reasons == ["missing-snapshot-checksum"]
    ));
}

// Ensure restore-readiness gating accepts plans with complete snapshot artifacts.
#[test]
fn run_restore_plan_require_restore_ready_accepts_ready_plan() {
    let root = temp_dir("canic-cli-restore-plan-ready");
    fs::create_dir_all(&root).expect("create temp root");
    let manifest_path = root.join("manifest.json");
    let out_path = root.join("plan.json");

    fs::write(
        &manifest_path,
        serde_json::to_vec(&restore_ready_manifest()).expect("serialize manifest"),
    )
    .expect("write manifest");

    run([
        OsString::from("plan"),
        OsString::from("--manifest"),
        OsString::from(manifest_path.as_os_str()),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--require-restore-ready"),
    ])
    .expect("restore-ready plan should pass");

    let plan: RestorePlan =
        serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(plan.readiness_summary.ready);
    assert!(plan.readiness_summary.reasons.is_empty());
}

// Ensure restore apply dry-run writes ordered operations from a plan.
#[test]
fn run_restore_apply_dry_run_writes_operations() {
    let root = temp_dir("canic-cli-restore-apply-dry-run");
    fs::create_dir_all(&root).expect("create temp root");
    let plan_path = root.join("restore-plan.json");
    let out_path = root.join("restore-apply-dry-run.json");
    let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");

    fs::write(
        &plan_path,
        serde_json::to_vec(&plan).expect("serialize plan"),
    )
    .expect("write plan");

    run([
        OsString::from("apply"),
        OsString::from("--plan"),
        OsString::from(plan_path.as_os_str()),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
    ])
    .expect("write apply dry-run");

    let dry_run: RestoreApplyDryRun =
        serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
            .expect("decode dry-run");
    let dry_run_json: serde_json::Value = serde_json::to_value(&dry_run).expect("encode dry-run");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(dry_run.dry_run_version, 1);
    assert_eq!(dry_run.backup_id.as_str(), "backup-test");
    assert!(dry_run.ready);
    assert_eq!(dry_run.member_count, 2);
    assert_eq!(dry_run.planned_snapshot_uploads, 2);
    assert_eq!(dry_run.planned_operations, 6);
    assert_eq!(dry_run.rendered_operations, 6);
    assert_eq!(dry_run_json["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(dry_run_json["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(dry_run_json["operation_counts"]["member_verifications"], 2);
    assert_eq!(dry_run_json["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(
        dry_run_json["operation_counts"]["verification_operations"],
        2
    );
    assert_eq!(
        dry_run_json["operations"][0]["operation"],
        "upload-snapshot"
    );
    assert_eq!(dry_run_json["operations"][2]["operation"], "verify-member");
    assert_eq!(dry_run_json["operations"][2]["verification_kind"], "status");
}

// Ensure restore apply dry-run can validate artifacts under a backup directory.
#[test]
fn run_restore_apply_dry_run_validates_backup_dir_artifacts() {
    let root = temp_dir("canic-cli-restore-apply-artifacts");
    fs::create_dir_all(&root).expect("create temp root");
    let plan_path = root.join("restore-plan.json");
    let out_path = root.join("restore-apply-dry-run.json");
    let journal_path = root.join("restore-apply-journal.json");
    let mut manifest = restore_ready_manifest();
    write_manifest_artifacts(&root, &mut manifest);
    let plan = RestorePlanner::plan(&manifest, None).expect("build plan");

    fs::write(
        &plan_path,
        serde_json::to_vec(&plan).expect("serialize plan"),
    )
    .expect("write plan");

    run([
        OsString::from("apply"),
        OsString::from("--plan"),
        OsString::from(plan_path.as_os_str()),
        OsString::from("--backup-dir"),
        OsString::from(root.as_os_str()),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from(out_path.as_os_str()),
        OsString::from("--journal-out"),
        OsString::from(journal_path.as_os_str()),
    ])
    .expect("write apply dry-run");
    let dry_run: RestoreApplyDryRun =
        serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
            .expect("decode dry-run");
    let validation = dry_run
        .artifact_validation
        .expect("artifact validation should be present");
    let journal_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).expect("read journal"))
            .expect("decode journal");
    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(validation.checked_members, 2);
    assert!(validation.artifacts_present);
    assert!(validation.checksums_verified);
    assert_eq!(validation.members_with_expected_checksums, 2);
    assert_eq!(journal_json["ready"], true);
    assert_eq!(journal_json["operation_count"], 6);
    assert_eq!(journal_json["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(journal_json["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(journal_json["operation_counts"]["member_verifications"], 2);
    assert_eq!(journal_json["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(
        journal_json["operation_counts"]["verification_operations"],
        2
    );
    assert_eq!(journal_json["ready_operations"], 6);
    assert_eq!(journal_json["blocked_operations"], 0);
    assert_eq!(journal_json["operations"][0]["state"], "ready");
}

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
        OsString::from("--dfx"),
        OsString::from("/tmp/dfx"),
        OsString::from("--network"),
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
    assert_eq!(dry_run["operation_counts"]["snapshot_uploads"], 2);
    assert_eq!(dry_run["operation_counts"]["snapshot_loads"], 2);
    assert_eq!(dry_run["operation_counts"]["member_verifications"], 2);
    assert_eq!(dry_run["operation_counts"]["fleet_verifications"], 0);
    assert_eq!(dry_run["operation_counts"]["verification_operations"], 2);
    assert_eq!(dry_run["progress"]["operation_count"], 6);
    assert_eq!(dry_run["progress"]["completed_operations"], 0);
    assert_eq!(dry_run["progress"]["remaining_operations"], 6);
    assert_eq!(dry_run["progress"]["transitionable_operations"], 6);
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
    assert_eq!(dry_run["command"]["program"], "/tmp/dfx");
    assert_eq!(
        dry_run["command"]["args"],
        json!([
            "canister",
            "--network",
            "local",
            "snapshot",
            "upload",
            "--dir",
            "/tmp/canic-cli-restore-artifacts/artifacts/root",
            ROOT
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
    assert_eq!(run_summary["ready_operations"], 6);
    assert_eq!(run_summary["attention_required"], false);
    assert_eq!(updated.pending_operations, 0);
    assert_eq!(updated.ready_operations, 6);
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
        OsString::from("--dfx"),
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
    let fake_dfx = write_fake_dfx_upload(&root, "target-snap-root");
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
        OsString::from("--dfx"),
        OsString::from(fake_dfx.as_os_str()),
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
        preview.command.expect("load command").args,
        vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "load".to_string(),
            ROOT.to_string(),
            "target-snap-root".to_string(),
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
        .run_restore_run(&["--execute", "--dfx", "/bin/true", "--max-steps", "1"])
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
        OsString::from("--dfx"),
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
            operation_count: 6,
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
        OsString::from("--dfx"),
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

// Build one manually ready apply journal for runner-focused CLI tests.
fn ready_apply_journal() -> RestoreApplyJournal {
    let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal.ready = true;
    journal.blocked_reasons = Vec::new();
    journal.backup_root = Some("/tmp/canic-cli-restore-artifacts".to_string());
    for operation in &mut journal.operations {
        operation.state = canic_backup::restore::RestoreApplyOperationState::Ready;
        operation.blocking_reasons = Vec::new();
    }
    journal.blocked_operations = 0;
    journal.ready_operations = journal.operation_count;
    journal.validate().expect("journal should validate");
    journal
}

// Build one valid manifest for restore planning tests.
fn valid_manifest() -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: "backup-test".to_string(),
        created_at: "2026-05-03T00:00:00Z".to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "0.30.1".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "fleet".to_string(),
                kind: BackupUnitKind::Subtree,
                roles: vec!["root".to_string(), "app".to_string()],
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![
                fleet_member("root", ROOT, None, IdentityMode::Fixed),
                fleet_member("app", CHILD, Some(ROOT), IdentityMode::Relocatable),
            ],
        },
        verification: VerificationPlan::default(),
    }
}

// Build one manifest whose restore readiness metadata is complete.
fn restore_ready_manifest() -> FleetBackupManifest {
    let mut manifest = valid_manifest();
    for member in &mut manifest.fleet.members {
        member.source_snapshot.module_hash = Some(HASH.to_string());
        member.source_snapshot.wasm_hash = Some(HASH.to_string());
        member.source_snapshot.checksum = Some(HASH.to_string());
    }
    manifest
}

// Build one valid manifest member.
fn fleet_member(
    role: &str,
    canister_id: &str,
    parent_canister_id: Option<&str>,
    identity_mode: IdentityMode,
) -> FleetMember {
    FleetMember {
        role: role.to_string(),
        canister_id: canister_id.to_string(),
        parent_canister_id: parent_canister_id.map(str::to_string),
        subnet_canister_id: Some(ROOT.to_string()),
        controller_hint: None,
        identity_mode,
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: vec![role.to_string()],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: format!("{role}-snapshot"),
            module_hash: None,
            wasm_hash: None,
            code_version: Some("v0.30.1".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
        },
    }
}

// Write a canonical backup layout whose journal checksums match the artifacts.
fn write_verified_layout(root: &Path, layout: &BackupLayout, manifest: &FleetBackupManifest) {
    layout.write_manifest(manifest).expect("write manifest");

    let artifacts = manifest
        .fleet
        .members
        .iter()
        .map(|member| {
            let bytes = format!("{} artifact", member.role);
            let artifact_path = root.join(&member.source_snapshot.artifact_path);
            if let Some(parent) = artifact_path.parent() {
                fs::create_dir_all(parent).expect("create artifact parent");
            }
            fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
            let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());

            ArtifactJournalEntry {
                canister_id: member.canister_id.clone(),
                snapshot_id: member.source_snapshot.snapshot_id.clone(),
                state: ArtifactState::Durable,
                temp_path: None,
                artifact_path: member.source_snapshot.artifact_path.clone(),
                checksum_algorithm: checksum.algorithm,
                checksum: Some(checksum.hash),
                updated_at: "2026-05-03T00:00:00Z".to_string(),
            }
        })
        .collect();

    layout
        .write_journal(&DownloadJournal {
            journal_version: 1,
            backup_id: manifest.backup_id.clone(),
            discovery_topology_hash: Some(manifest.fleet.discovery_topology_hash.clone()),
            pre_snapshot_topology_hash: Some(manifest.fleet.pre_snapshot_topology_hash.clone()),
            operation_metrics: canic_backup::journal::DownloadOperationMetrics::default(),
            artifacts,
        })
        .expect("write journal");
}

// Write artifact bytes and update the manifest checksums for apply validation.
fn write_manifest_artifacts(root: &Path, manifest: &mut FleetBackupManifest) {
    for member in &mut manifest.fleet.members {
        let bytes = format!("{} apply artifact", member.role);
        let artifact_path = root.join(&member.source_snapshot.artifact_path);
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent).expect("create artifact parent");
        }
        fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
        let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());
        member.source_snapshot.checksum = Some(checksum.hash);
    }
}

// Build a unique temporary directory.
fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

// Derive the runner sidecar lock path for assertions.
fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}
