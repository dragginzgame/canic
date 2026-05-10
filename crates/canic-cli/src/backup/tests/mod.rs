use crate::test_support::temp_dir;
use canic_backup::{
    artifacts::ArtifactChecksum,
    execution::BackupExecutionJournal,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure backup help stays at command-family level.
#[test]
fn backup_usage_lists_commands_without_nested_flag_dump() {
    let text = usage();

    assert!(text.contains("Usage: canic backup"));
    assert!(text.contains("create"));
    assert!(text.contains("list"));
    assert!(text.contains("verify"));
    assert!(text.contains("status"));
}

// Ensure backup create options keep live execution behind an explicit dry-run.
#[test]
fn parses_backup_create_options() {
    let options = BackupCreateOptions::parse([
        OsString::from("demo"),
        OsString::from("--subtree"),
        OsString::from("app"),
        OsString::from("--out"),
        OsString::from("backups/plan"),
        OsString::from("--dry-run"),
        OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::args::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.subtree, Some("app".to_string()));
    assert_eq!(options.out, Some(PathBuf::from("backups/plan")));
    assert!(options.dry_run);
    assert_eq!(options.network, "local");
    assert_eq!(options.icp, "/bin/icp");
}

// Ensure create without dry-run remains fail-closed until authority execution exists.
#[test]
fn backup_create_requires_dry_run() {
    let options = BackupCreateOptions {
        fleet: "demo".to_string(),
        subtree: None,
        out: None,
        dry_run: false,
        network: "local".to_string(),
        icp: "icp".to_string(),
    };

    let err = backup_create(&options).expect_err("non-dry-run create rejects");

    assert!(matches!(err, BackupCommandError::CreateRequiresDryRun));
}

// Ensure dry-run persistence writes a plan and matching execution journal.
#[test]
fn backup_create_dry_run_persists_plan_and_execution_journal() {
    let root = temp_dir("canic-cli-backup-create-plan");
    let plan = valid_backup_plan();

    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let layout = BackupLayout::new(root.clone());
    let read_plan = layout.read_backup_plan().expect("read backup plan");
    let journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let report = layout
        .verify_execution_integrity()
        .expect("verify execution integrity");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(read_plan.plan_id, plan.plan_id);
    assert_eq!(journal.plan_id, plan.plan_id);
    assert!(report.verified);
}

// Ensure backup list options default to the conventional backup root.
#[test]
fn parses_backup_list_options() {
    let options = BackupListOptions::parse([
        OsString::from("--dir"),
        OsString::from("snapshots"),
        OsString::from("--out"),
        OsString::from("backups.txt"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("snapshots"));
    assert_eq!(options.out, Some(PathBuf::from("backups.txt")));

    let default_options = BackupListOptions::parse([]).expect("parse default options");
    assert_eq!(default_options.dir, PathBuf::from("backups"));
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

// Ensure backup list scans manifest-bearing directories and renders reusable paths.
#[test]
fn backup_list_reads_backup_directories() {
    let root = temp_dir("canic-cli-backup-list");
    let first = root.join("fleet-demo-20260507-120000");
    let second = root.join("fleet-demo-20260507-130000");
    let planned = root.join("fleet-demo-20260511-001234");
    let ignored = root.join("not-a-backup");

    BackupLayout::new(first)
        .write_manifest(&valid_manifest_with("backup-old", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second)
        .write_manifest(&valid_manifest_with("backup-new", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");
    let mut plan = valid_backup_plan();
    plan.plan_id = "plan-demo-20260511-001234".to_string();
    plan.run_id = "run-demo-20260511-001234".to_string();
    let planned_layout = BackupLayout::new(planned);
    planned_layout
        .write_backup_plan(&plan)
        .expect("write planned backup");
    planned_layout
        .write_execution_journal(
            &BackupExecutionJournal::from_plan(&plan).expect("execution journal"),
        )
        .expect("write planned journal");
    fs::create_dir_all(&ignored).expect("create ignored dir");

    let options = BackupListOptions {
        dir: root.clone(),
        out: None,
    };
    let entries = backup_list(&options).expect("list backups");
    let rendered = render_backup_list(&entries);

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-new"));
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-old"));
    let dry_run = entries
        .iter()
        .find(|entry| entry.backup_id == "plan-demo-20260511-001234")
        .expect("dry-run entry");
    assert_eq!(dry_run.status, "dry-run");
    assert_eq!(dry_run.members, 1);
    assert_eq!(dry_run.created_at, "20260511-001234");
    assert!(rendered.contains("DIR"));
    assert!(rendered.contains("backup-new"));
    assert!(rendered.contains("dry-run"));
    assert!(rendered.contains("fleet-demo-20260507-130000"));
}

// Ensure backup list hides machine timestamp markers in table output.
#[test]
fn backup_list_formats_unix_created_at() {
    let entries = vec![BackupListEntry {
        dir: PathBuf::from("backups/fleet-demo-20240507-140000"),
        backup_id: "backup".to_string(),
        created_at: "unix:1715090400".to_string(),
        members: 7,
        status: "ok".to_string(),
    }];
    let rendered = render_backup_list(&entries);

    assert!(rendered.contains("07/05/2024 14:00"));
    assert!(!rendered.contains("unix:"));
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
    valid_manifest_with("backup-test", "2026-05-03T00:00:00Z")
}

// Build one valid manifest with caller-provided summary fields.
fn valid_manifest_with(backup_id: &str, created_at: &str) -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: backup_id.to_string(),
        created_at: created_at.to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "0.30.3".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "fleet".to_string(),
                kind: BackupUnitKind::Single,
                roles: vec!["root".to_string()],
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
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
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

// Build one backup plan for create dry-run persistence tests.
fn valid_backup_plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(CHILD.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce: HASH.to_string(),
        registry: &[
            RegistryEntry {
                pid: ROOT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
            },
        ],
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Declared),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Declared,
        ),
        quiescence_policy: canic_backup::plan::QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("backup plan")
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

// Write one artifact at the layout-relative path used by test journals.
fn write_artifact(root: &Path, bytes: &[u8]) -> ArtifactChecksum {
    let path = root.join("artifacts/root");
    fs::create_dir_all(path.parent().expect("artifact has parent")).expect("create artifacts");
    fs::write(&path, bytes).expect("write artifact");
    ArtifactChecksum::from_bytes(bytes)
}

use super::*;
