use canic_backup::{
    artifacts::ArtifactChecksum,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetBackupManifest,
        FleetMember, FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
    restore::{RestoreMemberState, RestorePlan, RestoreStatus},
};
use serde_json::json;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT: &str = "aaaaa-aa";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure backup preflight options parse the intended command shape.
#[test]
fn parses_backup_preflight_options() {
    let options = BackupPreflightOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out-dir"),
        OsString::from("reports/run"),
        OsString::from("--mapping"),
        OsString::from("mapping.json"),
        OsString::from("--require-design"),
        OsString::from("--require-restore-ready"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out_dir, PathBuf::from("reports/run"));
    assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
    assert!(options.require_design_v1);
    assert!(options.require_restore_ready);
}

// Ensure backup smoke options parse the canonical no-mutation wrapper shape.
#[test]
fn parses_backup_smoke_options() {
    let options = BackupSmokeOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out-dir"),
        OsString::from("smoke/run"),
        OsString::from("--mapping"),
        OsString::from("mapping.json"),
        OsString::from("--dfx"),
        OsString::from("/bin/true"),
        OsString::from("--network"),
        OsString::from("local"),
        OsString::from("--require-design"),
        OsString::from("--require-restore-ready"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out_dir, PathBuf::from("smoke/run"));
    assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
    assert_eq!(options.dfx, "/bin/true");
    assert_eq!(options.network, Some("local".to_string()));
    assert!(options.require_design_v1);
    assert!(options.require_restore_ready);
}

// Ensure backup help stays at command-family level.
#[test]
fn backup_usage_lists_commands_without_nested_flag_dump() {
    let text = usage();

    assert!(text.contains("usage: canic backup <command> [<args>]"));
    assert!(text.contains("smoke"));
    assert!(text.contains("preflight"));
    assert!(text.contains("verify"));
    assert!(!text.contains("--require-restore-ready"));
    assert!(!text.contains("--require-design"));
}

// Ensure preflight writes the standard no-mutation report bundle.
#[test]
fn backup_preflight_writes_standard_reports() {
    let root = temp_dir("canic-cli-backup-preflight");
    let out_dir = root.join("reports");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());
    let checksum = write_artifact(&backup_dir, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let options = BackupPreflightOptions {
        dir: backup_dir,
        out_dir: out_dir.clone(),
        mapping: None,
        require_design_v1: false,
        require_restore_ready: false,
    };
    let report = backup_preflight(&options).expect("run preflight");

    assert_eq!(report.status, "ready");
    assert_eq!(report.backup_id, "backup-test");
    assert_eq!(report.source_environment, "local");
    assert_eq!(report.source_root_canister, ROOT);
    assert_eq!(report.topology_hash, HASH);
    assert_eq!(report.mapping_path, None);
    assert!(report.journal_complete);
    assert_eq!(
        report.journal_operation_metrics,
        DownloadOperationMetrics::default()
    );
    assert_eq!(report.inspection_status, "ready");
    assert_eq!(report.provenance_status, "consistent");
    assert_eq!(report.backup_id_status, "matched");
    assert_eq!(report.topology_receipts_status, "matched");
    assert_eq!(report.topology_mismatch_count, 0);
    assert!(report.integrity_verified);
    assert!(!report.manifest_design_v1_ready);
    assert_eq!(report.manifest_members, 1);
    assert_eq!(report.backup_unit_count, 1);
    assert_eq!(report.restore_plan_members, 1);
    assert!(!report.restore_mapping_supplied);
    assert!(!report.restore_all_sources_mapped);
    assert_preflight_report_restore_counts(&report);
    assert!(out_dir.join("manifest-validation.json").exists());
    assert!(out_dir.join("backup-status.json").exists());
    assert!(out_dir.join("backup-inspection.json").exists());
    assert!(out_dir.join("backup-provenance.json").exists());
    assert!(out_dir.join("backup-integrity.json").exists());
    assert!(out_dir.join("restore-plan.json").exists());
    assert!(out_dir.join("restore-status.json").exists());
    assert!(out_dir.join("preflight-summary.json").exists());

    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("preflight-summary.json")).expect("read summary"),
    )
    .expect("decode summary");
    let manifest_validation: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("manifest-validation.json")).expect("read manifest summary"),
    )
    .expect("decode manifest summary");
    let restore_status: RestoreStatus = serde_json::from_slice(
        &fs::read(out_dir.join("restore-status.json")).expect("read restore status"),
    )
    .expect("decode restore status");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_preflight_summary_matches_report(&summary, &report);
    assert_eq!(restore_status.status_version, 1);
    assert_eq!(restore_status.backup_id.as_str(), report.backup_id.as_str());
    assert_eq!(restore_status.member_count, report.restore_plan_members);
    assert_eq!(restore_status.phase_count, report.restore_phase_count);
    assert_eq!(
        restore_status.phases[0].members[0].state,
        RestoreMemberState::Planned
    );
    assert_eq!(manifest_validation["backup_unit_count"], 1);
    assert_eq!(manifest_validation["consistency_mode"], "crash-consistent");
    assert_eq!(
        manifest_validation["topology_validation_status"],
        "validated"
    );
    assert_eq!(
        manifest_validation["backup_unit_kinds"]["subtree_rooted"],
        1
    );
    assert_eq!(
        manifest_validation["backup_units"][0]["kind"],
        "subtree-rooted"
    );
    assert_eq!(
        manifest_validation["design_conformance"]["design_v1_ready"],
        false
    );
}

// Ensure restore-readiness gating happens after writing the report bundle.
#[test]
fn backup_preflight_require_restore_ready_writes_reports_then_fails() {
    let root = temp_dir("canic-cli-backup-preflight-require-restore-ready");
    let out_dir = root.join("reports");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());
    let checksum = write_artifact(&backup_dir, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let options = BackupPreflightOptions {
        dir: backup_dir,
        out_dir: out_dir.clone(),
        mapping: None,
        require_design_v1: false,
        require_restore_ready: true,
    };

    let err = backup_preflight(&options).expect_err("restore readiness should be enforced");

    assert!(out_dir.join("preflight-summary.json").exists());
    assert!(out_dir.join("restore-status.json").exists());
    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("preflight-summary.json")).expect("read summary"),
    )
    .expect("decode summary");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(summary["restore_ready"], false);
    assert!(matches!(
        err,
        BackupCommandError::RestoreNotReady {
            reasons,
            ..
        } if reasons == [
            "missing-module-hash",
            "missing-wasm-hash",
            "missing-snapshot-checksum"
        ]
    ));
}

// Ensure design gating happens after writing the report bundle.
#[test]
fn backup_preflight_require_design_v1_writes_reports_then_fails() {
    let root = temp_dir("canic-cli-backup-preflight-require-design");
    let out_dir = root.join("reports");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());
    let checksum = write_artifact(&backup_dir, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let options = BackupPreflightOptions {
        dir: backup_dir,
        out_dir: out_dir.clone(),
        mapping: None,
        require_design_v1: true,
        require_restore_ready: false,
    };

    let err = backup_preflight(&options).expect_err("design readiness should be enforced");

    assert!(out_dir.join("preflight-summary.json").exists());
    assert!(out_dir.join("manifest-validation.json").exists());
    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("preflight-summary.json")).expect("read summary"),
    )
    .expect("decode summary");
    let manifest_validation: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("manifest-validation.json")).expect("read manifest summary"),
    )
    .expect("decode manifest summary");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(summary["manifest_design_v1_ready"], false);
    assert_eq!(
        manifest_validation["design_conformance"]["design_v1_ready"],
        false
    );
    assert!(matches!(
        err,
        BackupCommandError::DesignConformanceNotReady { .. }
    ));
}

// Ensure restore-readiness gating accepts fully populated preflight reports.
#[test]
fn backup_preflight_require_restore_ready_accepts_ready_report() {
    let root = temp_dir("canic-cli-backup-preflight-ready");
    let out_dir = root.join("reports");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());
    let checksum = write_artifact(&backup_dir, b"root artifact");

    layout
        .write_manifest(&restore_ready_manifest(&checksum.hash))
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let options = BackupPreflightOptions {
        dir: backup_dir,
        out_dir: out_dir.clone(),
        mapping: None,
        require_design_v1: true,
        require_restore_ready: true,
    };

    let report = backup_preflight(&options).expect("ready preflight should pass");
    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("preflight-summary.json")).expect("read summary"),
    )
    .expect("decode summary");
    let manifest_validation: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("manifest-validation.json")).expect("read manifest summary"),
    )
    .expect("decode manifest summary");
    let restore_plan: RestorePlan =
        serde_json::from_slice(&fs::read(out_dir.join("restore-plan.json")).expect("read plan"))
            .expect("decode restore plan");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(report.manifest_design_v1_ready);
    assert!(report.restore_ready);
    assert!(report.restore_readiness_reasons.is_empty());
    assert_eq!(summary["restore_ready"], true);
    assert_eq!(summary["manifest_design_v1_ready"], true);
    assert_eq!(
        manifest_validation["design_conformance"]["design_v1_ready"],
        true
    );
    assert!(
        restore_plan
            .design_conformance
            .as_ref()
            .expect("restore plan should include design conformance")
            .design_v1_ready
    );
    assert_eq!(summary["restore_readiness_reasons"], json!([]));
    assert_eq!(
        summary["restore_status_path"],
        out_dir.join("restore-status.json").display().to_string()
    );
}

// Ensure backup smoke writes the post-capture release smoke bundle.
#[test]
fn backup_smoke_writes_release_bundle() {
    let root = temp_dir("canic-cli-backup-smoke");
    let out_dir = root.join("smoke");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());
    let checksum = write_artifact(&backup_dir, b"root artifact");

    layout
        .write_manifest(&restore_ready_manifest(&checksum.hash))
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let options = BackupSmokeOptions {
        dir: backup_dir,
        out_dir: out_dir.clone(),
        mapping: None,
        dfx: "/bin/true".to_string(),
        network: Some("local".to_string()),
        require_design_v1: true,
        require_restore_ready: true,
    };

    let report = backup_smoke(&options).expect("smoke should pass");
    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("smoke-summary.json")).expect("read smoke summary"),
    )
    .expect("decode smoke summary");
    let runner_preview: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("restore-run-dry-run.json")).expect("read runner preview"),
    )
    .expect("decode runner preview");

    assert_eq!(report.status, "ready");
    assert_eq!(report.backup_id, "backup-test");
    assert!(report.manifest_design_v1_ready);
    assert!(report.restore_ready);
    assert!(report.runner_preview_written);
    assert!(out_dir.join("preflight/preflight-summary.json").exists());
    assert!(out_dir.join("preflight/restore-plan.json").exists());
    assert!(out_dir.join("preflight/restore-status.json").exists());
    assert!(out_dir.join("restore-apply-dry-run.json").exists());
    assert!(out_dir.join("restore-apply-journal.json").exists());
    assert!(out_dir.join("restore-run-dry-run.json").exists());
    assert_eq!(summary["status"], "ready");
    assert_eq!(summary["restore_ready"], true);
    assert_eq!(summary["manifest_design_v1_ready"], true);
    assert_eq!(summary["runner_preview_written"], true);
    assert_eq!(runner_preview["run_mode"], "dry-run");
    assert_eq!(runner_preview["dry_run"], true);
    assert_eq!(runner_preview["operation_receipt_count"], 0);

    fs::remove_dir_all(root).expect("remove temp root");
}

// Verify restore summary counts copied out of the generated restore plan.
fn assert_preflight_report_restore_counts(report: &BackupPreflightReport) {
    assert_eq!(report.restore_fixed_members, 1);
    assert_eq!(report.restore_relocatable_members, 0);
    assert_eq!(report.restore_in_place_members, 1);
    assert_eq!(report.restore_mapped_members, 0);
    assert_eq!(report.restore_remapped_members, 0);
    assert!(!report.restore_ready);
    assert_eq!(
        report.restore_readiness_reasons,
        [
            "missing-module-hash",
            "missing-wasm-hash",
            "missing-snapshot-checksum"
        ]
    );
    assert!(!report.restore_all_members_have_module_hash);
    assert!(!report.restore_all_members_have_wasm_hash);
    assert!(report.restore_all_members_have_code_version);
    assert!(!report.restore_all_members_have_checksum);
    assert_eq!(report.restore_members_with_module_hash, 0);
    assert_eq!(report.restore_members_with_wasm_hash, 0);
    assert_eq!(report.restore_members_with_code_version, 1);
    assert_eq!(report.restore_members_with_checksum, 0);
    assert!(report.restore_verification_required);
    assert!(report.restore_all_members_have_checks);
    assert_eq!(report.restore_fleet_checks, 0);
    assert_eq!(report.restore_member_check_groups, 0);
    assert_eq!(report.restore_member_checks, 1);
    assert_eq!(report.restore_members_with_checks, 1);
    assert_eq!(report.restore_total_checks, 1);
    assert_eq!(report.restore_planned_snapshot_uploads, 1);
    assert_eq!(report.restore_planned_snapshot_loads, 1);
    assert_eq!(report.restore_planned_code_reinstalls, 0);
    assert_eq!(report.restore_planned_verification_checks, 1);
    assert_eq!(report.restore_planned_operations, 3);
    assert_eq!(report.restore_planned_phases, 1);
    assert_eq!(report.restore_phase_count, 1);
    assert_eq!(report.restore_dependency_free_members, 1);
    assert_eq!(report.restore_in_group_parent_edges, 0);
    assert_eq!(report.restore_cross_group_parent_edges, 0);
}

// Compare preflight summary JSON with the in-memory report.
fn assert_preflight_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_preflight_source_summary_matches_report(summary, report);
    assert_preflight_restore_identity_summary_matches_report(summary, report);
    assert_preflight_restore_readiness_summary_matches_report(summary, report);
    assert_preflight_restore_snapshot_summary_matches_report(summary, report);
    assert_preflight_restore_verification_summary_matches_report(summary, report);
    assert_preflight_restore_operation_summary_matches_report(summary, report);
    assert_preflight_restore_ordering_summary_matches_report(summary, report);
    assert_preflight_path_summary_matches_report(summary, report);
}

// Compare source and validation summary JSON fields with the in-memory report.
fn assert_preflight_source_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(summary["status"], report.status);
    assert_eq!(summary["backup_id"], report.backup_id);
    assert_eq!(summary["source_environment"], report.source_environment);
    assert_eq!(summary["source_root_canister"], report.source_root_canister);
    assert_eq!(summary["topology_hash"], report.topology_hash);
    assert_eq!(summary["journal_complete"], report.journal_complete);
    assert_eq!(
        summary["journal_operation_metrics"],
        json!(report.journal_operation_metrics)
    );
    assert_eq!(summary["inspection_status"], report.inspection_status);
    assert_eq!(summary["provenance_status"], report.provenance_status);
    assert_eq!(summary["backup_id_status"], report.backup_id_status);
    assert_eq!(
        summary["topology_receipts_status"],
        report.topology_receipts_status
    );
    assert_eq!(
        summary["topology_mismatch_count"],
        report.topology_mismatch_count
    );
    assert_eq!(summary["integrity_verified"], report.integrity_verified);
    assert_eq!(
        summary["manifest_design_v1_ready"],
        report.manifest_design_v1_ready
    );
    assert_eq!(summary["manifest_members"], report.manifest_members);
    assert_eq!(summary["backup_unit_count"], report.backup_unit_count);
    assert_eq!(summary["restore_plan_members"], report.restore_plan_members);
    assert_eq!(
        summary["restore_mapping_supplied"],
        report.restore_mapping_supplied
    );
    assert_eq!(
        summary["restore_all_sources_mapped"],
        report.restore_all_sources_mapped
    );
}

// Compare restore identity summary JSON fields with the in-memory report.
fn assert_preflight_restore_identity_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(
        summary["restore_fixed_members"],
        report.restore_fixed_members
    );
    assert_eq!(
        summary["restore_relocatable_members"],
        report.restore_relocatable_members
    );
    assert_eq!(
        summary["restore_in_place_members"],
        report.restore_in_place_members
    );
    assert_eq!(
        summary["restore_mapped_members"],
        report.restore_mapped_members
    );
    assert_eq!(
        summary["restore_remapped_members"],
        report.restore_remapped_members
    );
}

// Compare restore readiness summary JSON fields with the in-memory report.
fn assert_preflight_restore_readiness_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(summary["restore_ready"], report.restore_ready);
    assert_eq!(
        summary["restore_readiness_reasons"],
        json!(report.restore_readiness_reasons)
    );
}

// Compare restore snapshot summary JSON fields with the in-memory report.
fn assert_preflight_restore_snapshot_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(
        summary["restore_all_members_have_module_hash"],
        report.restore_all_members_have_module_hash
    );
    assert_eq!(
        summary["restore_all_members_have_wasm_hash"],
        report.restore_all_members_have_wasm_hash
    );
    assert_eq!(
        summary["restore_all_members_have_code_version"],
        report.restore_all_members_have_code_version
    );
    assert_eq!(
        summary["restore_all_members_have_checksum"],
        report.restore_all_members_have_checksum
    );
    assert_eq!(
        summary["restore_members_with_module_hash"],
        report.restore_members_with_module_hash
    );
    assert_eq!(
        summary["restore_members_with_wasm_hash"],
        report.restore_members_with_wasm_hash
    );
    assert_eq!(
        summary["restore_members_with_code_version"],
        report.restore_members_with_code_version
    );
    assert_eq!(
        summary["restore_members_with_checksum"],
        report.restore_members_with_checksum
    );
}

// Compare restore verification summary JSON fields with the in-memory report.
fn assert_preflight_restore_verification_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(
        summary["restore_verification_required"],
        report.restore_verification_required
    );
    assert_eq!(
        summary["restore_all_members_have_checks"],
        report.restore_all_members_have_checks
    );
    assert_eq!(summary["restore_fleet_checks"], report.restore_fleet_checks);
    assert_eq!(
        summary["restore_member_check_groups"],
        report.restore_member_check_groups
    );
    assert_eq!(
        summary["restore_member_checks"],
        report.restore_member_checks
    );
    assert_eq!(
        summary["restore_members_with_checks"],
        report.restore_members_with_checks
    );
    assert_eq!(summary["restore_total_checks"], report.restore_total_checks);
}

// Compare restore operation summary JSON fields with the in-memory report.
fn assert_preflight_restore_operation_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(
        summary["restore_planned_snapshot_uploads"],
        report.restore_planned_snapshot_uploads
    );
    assert_eq!(
        summary["restore_planned_snapshot_loads"],
        report.restore_planned_snapshot_loads
    );
    assert_eq!(
        summary["restore_planned_code_reinstalls"],
        report.restore_planned_code_reinstalls
    );
    assert_eq!(
        summary["restore_planned_verification_checks"],
        report.restore_planned_verification_checks
    );
    assert_eq!(
        summary["restore_planned_operations"],
        report.restore_planned_operations
    );
    assert_eq!(
        summary["restore_planned_phases"],
        report.restore_planned_phases
    );
}

// Compare restore ordering summary JSON fields with the in-memory report.
fn assert_preflight_restore_ordering_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(summary["restore_phase_count"], report.restore_phase_count);
    assert_eq!(
        summary["restore_dependency_free_members"],
        report.restore_dependency_free_members
    );
    assert_eq!(
        summary["restore_in_group_parent_edges"],
        report.restore_in_group_parent_edges
    );
    assert_eq!(
        summary["restore_cross_group_parent_edges"],
        report.restore_cross_group_parent_edges
    );
}

// Compare generated report path JSON fields with the in-memory report.
fn assert_preflight_path_summary_matches_report(
    summary: &serde_json::Value,
    report: &BackupPreflightReport,
) {
    assert_eq!(
        summary["manifest_validation_path"],
        report.manifest_validation_path
    );
    assert_eq!(summary["backup_status_path"], report.backup_status_path);
    assert_eq!(
        summary["backup_inspection_path"],
        report.backup_inspection_path
    );
    assert_eq!(
        summary["backup_provenance_path"],
        report.backup_provenance_path
    );
    assert_eq!(
        summary["backup_integrity_path"],
        report.backup_integrity_path
    );
    assert_eq!(summary["restore_plan_path"], report.restore_plan_path);
    assert_eq!(summary["restore_status_path"], report.restore_status_path);
    assert_eq!(
        summary["preflight_summary_path"],
        report.preflight_summary_path
    );
}

// Ensure preflight stops on incomplete journals before claiming readiness.
#[test]
fn backup_preflight_rejects_incomplete_journal() {
    let root = temp_dir("canic-cli-backup-preflight-incomplete");
    let out_dir = root.join("reports");
    let backup_dir = root.join("backup");
    let layout = BackupLayout::new(backup_dir.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&created_journal())
        .expect("write journal");

    let options = BackupPreflightOptions {
        dir: backup_dir,
        out_dir,
        mapping: None,
        require_design_v1: false,
        require_restore_ready: false,
    };

    let err = backup_preflight(&options).expect_err("incomplete journal should fail");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(matches!(
        err,
        BackupCommandError::IncompleteJournal {
            pending_artifacts: 1,
            total_artifacts: 1,
            ..
        }
    ));
}

// Ensure backup verification options parse the intended command shape.
#[test]
fn parses_backup_verify_options() {
    let options = BackupVerifyOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("report.json"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out, Some(PathBuf::from("report.json")));
}

// Ensure backup inspection options parse the intended command shape.
#[test]
fn parses_backup_inspect_options() {
    let options = BackupInspectOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("inspect.json"),
        OsString::from("--require-ready"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out, Some(PathBuf::from("inspect.json")));
    assert!(options.require_ready);
}

// Ensure backup provenance options parse the intended command shape.
#[test]
fn parses_backup_provenance_options() {
    let options = BackupProvenanceOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("provenance.json"),
        OsString::from("--require-consistent"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out, Some(PathBuf::from("provenance.json")));
    assert!(options.require_consistent);
}

// Ensure backup status options parse the intended command shape.
#[test]
fn parses_backup_status_options() {
    let options = BackupStatusOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("status.json"),
        OsString::from("--require-complete"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("backups/run"));
    assert_eq!(options.out, Some(PathBuf::from("status.json")));
    assert!(options.require_complete);
}

// Ensure backup status reads the journal and reports resume actions.
#[test]
fn backup_status_reads_journal_resume_report() {
    let root = temp_dir("canic-cli-backup-status");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_journal(&journal_with_checksum(HASH.to_string()))
        .expect("write journal");

    let options = BackupStatusOptions {
        dir: root.clone(),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read backup status");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.backup_id, "backup-test");
    assert_eq!(report.total_artifacts, 1);
    assert!(report.is_complete);
    assert_eq!(report.pending_artifacts, 0);
    assert_eq!(report.counts.skip, 1);
}

// Ensure backup inspection reports manifest and journal agreement.
#[test]
fn inspect_backup_reads_layout_metadata() {
    let root = temp_dir("canic-cli-backup-inspect");
    let layout = BackupLayout::new(root.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(HASH.to_string()))
        .expect("write journal");

    let options = BackupInspectOptions {
        dir: root.clone(),
        out: None,
        require_ready: false,
    };
    let report = inspect_backup(&options).expect("inspect backup");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.backup_id, "backup-test");
    assert!(report.backup_id_matches);
    assert!(report.journal_complete);
    assert!(report.ready_for_verify);
    assert!(report.topology_receipt_mismatches.is_empty());
    assert_eq!(report.matched_artifacts, 1);
}

// Ensure backup provenance reports manifest and journal audit metadata.
#[test]
fn backup_provenance_reads_layout_metadata() {
    let root = temp_dir("canic-cli-backup-provenance");
    let layout = BackupLayout::new(root.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(HASH.to_string()))
        .expect("write journal");

    let options = BackupProvenanceOptions {
        dir: root.clone(),
        out: None,
        require_consistent: false,
    };
    let report = backup_provenance(&options).expect("read provenance");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.backup_id, "backup-test");
    assert!(report.backup_id_matches);
    assert_eq!(report.source_environment, "local");
    assert_eq!(report.discovery_topology_hash, HASH);
    assert!(report.topology_receipts_match);
    assert!(report.topology_receipt_mismatches.is_empty());
    assert_eq!(report.backup_unit_count, 1);
    assert_eq!(report.member_count, 1);
    assert_eq!(report.backup_units[0].kind, "subtree-rooted");
    assert_eq!(report.members[0].canister_id, ROOT);
    assert_eq!(report.members[0].snapshot_id, "root-snapshot");
    assert_eq!(report.members[0].journal_state, Some("Durable".to_string()));
}

// Ensure require-consistent accepts matching provenance reports.
#[test]
fn require_consistent_accepts_matching_provenance() {
    let options = BackupProvenanceOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_consistent: true,
    };
    let report = ready_provenance_report();

    enforce_provenance_requirements(&options, &report).expect("matching provenance should pass");
}

// Ensure require-consistent rejects backup ID or topology receipt drift.
#[test]
fn require_consistent_rejects_provenance_drift() {
    let options = BackupProvenanceOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_consistent: true,
    };
    let mut report = ready_provenance_report();
    report.backup_id_matches = false;
    report.journal_backup_id = "other-backup".to_string();
    report.topology_receipts_match = false;
    report
        .topology_receipt_mismatches
        .push(canic_backup::persistence::TopologyReceiptMismatch {
            field: "pre_snapshot_topology_hash".to_string(),
            manifest: HASH.to_string(),
            journal: None,
        });

    let err = enforce_provenance_requirements(&options, &report)
        .expect_err("provenance drift should fail");

    assert!(matches!(
        err,
        BackupCommandError::ProvenanceNotConsistent {
            backup_id_matches: false,
            topology_receipts_match: false,
            topology_mismatches: 1,
            ..
        }
    ));
}

// Ensure require-ready accepts inspection reports ready for verification.
#[test]
fn require_ready_accepts_ready_inspection() {
    let options = BackupInspectOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_ready: true,
    };
    let report = ready_inspection_report();

    enforce_inspection_requirements(&options, &report).expect("ready inspection should pass");
}

// Ensure require-ready rejects inspection reports with metadata drift.
#[test]
fn require_ready_rejects_unready_inspection() {
    let options = BackupInspectOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_ready: true,
    };
    let mut report = ready_inspection_report();
    report.ready_for_verify = false;
    report
        .path_mismatches
        .push(canic_backup::persistence::ArtifactPathMismatch {
            canister_id: ROOT.to_string(),
            snapshot_id: "root-snapshot".to_string(),
            manifest: "artifacts/root".to_string(),
            journal: "artifacts/other-root".to_string(),
        });

    let err = enforce_inspection_requirements(&options, &report)
        .expect_err("unready inspection should fail");

    assert!(matches!(
        err,
        BackupCommandError::InspectionNotReady {
            path_mismatches: 1,
            ..
        }
    ));
}

// Ensure require-ready rejects topology receipt drift.
#[test]
fn require_ready_rejects_topology_receipt_drift() {
    let options = BackupInspectOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_ready: true,
    };
    let mut report = ready_inspection_report();
    report.ready_for_verify = false;
    report
        .topology_receipt_mismatches
        .push(canic_backup::persistence::TopologyReceiptMismatch {
            field: "discovery_topology_hash".to_string(),
            manifest: HASH.to_string(),
            journal: None,
        });

    let err = enforce_inspection_requirements(&options, &report)
        .expect_err("topology receipt drift should fail");

    assert!(matches!(
        err,
        BackupCommandError::InspectionNotReady {
            topology_receipts_match: false,
            topology_mismatches: 1,
            ..
        }
    ));
}

// Ensure require-complete accepts already durable backup journals.
#[test]
fn require_complete_accepts_complete_status() {
    let options = BackupStatusOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_complete: true,
    };
    let report = journal_with_checksum(HASH.to_string()).resume_report();

    enforce_status_requirements(&options, &report).expect("complete status should pass");
}

// Ensure require-complete rejects journals that still need resume work.
#[test]
fn require_complete_rejects_incomplete_status() {
    let options = BackupStatusOptions {
        dir: PathBuf::from("unused"),
        out: None,
        require_complete: true,
    };
    let report = created_journal().resume_report();

    let err =
        enforce_status_requirements(&options, &report).expect_err("incomplete status should fail");

    assert!(matches!(
        err,
        BackupCommandError::IncompleteJournal {
            pending_artifacts: 1,
            total_artifacts: 1,
            ..
        }
    ));
}

// Ensure the CLI verification path reads a layout and returns an integrity report.
#[test]
fn verify_backup_reads_layout_and_artifacts() {
    let root = temp_dir("canic-cli-backup-verify");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash.clone()))
        .expect("write journal");

    let options = BackupVerifyOptions {
        dir: root.clone(),
        out: None,
    };
    let report = verify_backup(&options).expect("verify backup");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.backup_id, "backup-test");
    assert!(report.verified);
    assert_eq!(report.durable_artifacts, 1);
    assert_eq!(report.artifacts[0].checksum, checksum.hash);
}

// Build one valid manifest for CLI verification tests.
fn valid_manifest() -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: "backup-test".to_string(),
        created_at: "2026-05-03T00:00:00Z".to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "0.30.3".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            mode: ConsistencyMode::CrashConsistent,
            backup_units: vec![BackupUnit {
                unit_id: "fleet".to_string(),
                kind: BackupUnitKind::SubtreeRooted,
                roles: vec!["root".to_string()],
                consistency_reason: None,
                dependency_closure: Vec::new(),
                topology_validation: "subtree-closed".to_string(),
                quiescence_strategy: None,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![fleet_member()],
        },
        verification: VerificationPlan::default(),
    }
}

// Build one valid manifest member.
fn fleet_member() -> FleetMember {
    FleetMember {
        role: "root".to_string(),
        canister_id: ROOT.to_string(),
        parent_canister_id: None,
        subnet_canister_id: Some(ROOT.to_string()),
        controller_hint: None,
        identity_mode: IdentityMode::Fixed,
        restore_group: 1,
        verification_class: "basic".to_string(),
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            method: None,
            roles: vec!["root".to_string()],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: "root-snapshot".to_string(),
            module_hash: None,
            wasm_hash: None,
            code_version: Some("v0.30.3".to_string()),
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
        },
    }
}

// Build one manifest whose restore readiness metadata is complete.
fn restore_ready_manifest(checksum: &str) -> FleetBackupManifest {
    let mut manifest = valid_manifest();
    let snapshot = &mut manifest.fleet.members[0].source_snapshot;
    snapshot.module_hash = Some(HASH.to_string());
    snapshot.wasm_hash = Some(HASH.to_string());
    snapshot.checksum = Some(checksum.to_string());
    manifest
}

// Build one durable journal with a caller-provided checksum.
fn journal_with_checksum(checksum: String) -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: "backup-test".to_string(),
        discovery_topology_hash: Some(HASH.to_string()),
        pre_snapshot_topology_hash: Some(HASH.to_string()),
        operation_metrics: canic_backup::journal::DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: ROOT.to_string(),
            snapshot_id: "root-snapshot".to_string(),
            state: ArtifactState::Durable,
            temp_path: None,
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(checksum),
            updated_at: "2026-05-03T00:00:00Z".to_string(),
        }],
    }
}

// Build one incomplete journal that still needs artifact download work.
fn created_journal() -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: "backup-test".to_string(),
        discovery_topology_hash: Some(HASH.to_string()),
        pre_snapshot_topology_hash: Some(HASH.to_string()),
        operation_metrics: canic_backup::journal::DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: ROOT.to_string(),
            snapshot_id: "root-snapshot".to_string(),
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: "2026-05-03T00:00:00Z".to_string(),
        }],
    }
}

// Build one ready inspection report for requirement tests.
fn ready_inspection_report() -> BackupInspectionReport {
    BackupInspectionReport {
        backup_id: "backup-test".to_string(),
        manifest_backup_id: "backup-test".to_string(),
        journal_backup_id: "backup-test".to_string(),
        backup_id_matches: true,
        journal_complete: true,
        ready_for_verify: true,
        manifest_members: 1,
        journal_artifacts: 1,
        matched_artifacts: 1,
        topology_receipt_mismatches: Vec::new(),
        missing_journal_artifacts: Vec::new(),
        unexpected_journal_artifacts: Vec::new(),
        path_mismatches: Vec::new(),
        checksum_mismatches: Vec::new(),
    }
}

// Build one matching provenance report for requirement tests.
fn ready_provenance_report() -> BackupProvenanceReport {
    BackupProvenanceReport {
        backup_id: "backup-test".to_string(),
        manifest_backup_id: "backup-test".to_string(),
        journal_backup_id: "backup-test".to_string(),
        backup_id_matches: true,
        manifest_version: 1,
        journal_version: 1,
        created_at: "2026-05-03T00:00:00Z".to_string(),
        tool_name: "canic".to_string(),
        tool_version: "0.30.12".to_string(),
        source_environment: "local".to_string(),
        source_root_canister: ROOT.to_string(),
        topology_hash_algorithm: "sha256".to_string(),
        topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
        discovery_topology_hash: HASH.to_string(),
        pre_snapshot_topology_hash: HASH.to_string(),
        accepted_topology_hash: HASH.to_string(),
        journal_discovery_topology_hash: Some(HASH.to_string()),
        journal_pre_snapshot_topology_hash: Some(HASH.to_string()),
        topology_receipts_match: true,
        topology_receipt_mismatches: Vec::new(),
        backup_unit_count: 1,
        member_count: 1,
        consistency_mode: "crash-consistent".to_string(),
        backup_units: Vec::new(),
        members: Vec::new(),
    }
}

// Write one artifact at the layout-relative path used by test journals.
fn write_artifact(root: &Path, bytes: &[u8]) -> ArtifactChecksum {
    let path = root.join("artifacts/root");
    fs::create_dir_all(path.parent().expect("artifact has parent")).expect("create artifacts");
    fs::write(&path, bytes).expect("write artifact");
    ArtifactChecksum::from_bytes(bytes)
}

// Build a unique temporary directory.
fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
use super::*;
