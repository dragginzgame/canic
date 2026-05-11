use super::*;
use crate::test_support::temp_dir;
use crate::{
    discovery::RegistryEntry,
    execution::BackupExecutionJournal,
    journal::{ArtifactJournalEntry, ArtifactState},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetMember, FleetSection, IdentityMode,
        SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck, VerificationPlan,
    },
    plan::{
        AuthorityEvidence, BackupPlan, BackupPlanBuildInput, BackupScopeKind, ControlAuthority,
        SnapshotReadAuthority, build_backup_plan,
    },
};
use std::fs;

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure manifest writes create parent dirs and round-trip through validation.
#[test]
fn manifest_round_trips_through_layout() {
    let root = temp_dir("canic-backup-manifest-layout");
    let layout = BackupLayout::new(root.clone());
    let manifest = valid_manifest();

    layout
        .write_manifest(&manifest)
        .expect("write manifest atomically");
    let read = layout.read_manifest().expect("read manifest");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(read.backup_id, manifest.backup_id);
}

// Ensure journal writes create parent dirs and round-trip through validation.
#[test]
fn journal_round_trips_through_layout() {
    let root = temp_dir("canic-backup-journal-layout");
    let layout = BackupLayout::new(root.clone());
    let journal = valid_journal();

    layout
        .write_journal(&journal)
        .expect("write journal atomically");
    let read = layout.read_journal().expect("read journal");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(read.backup_id, journal.backup_id);
}

// Ensure backup plans write atomically and round-trip through validation.
#[test]
fn backup_plan_round_trips_through_layout() {
    let root = temp_dir("canic-backup-plan-layout");
    let layout = BackupLayout::new(root.clone());
    let plan = valid_backup_plan();

    layout
        .write_backup_plan(&plan)
        .expect("write backup plan atomically");
    let read = layout.read_backup_plan().expect("read backup plan");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(read.plan_id, plan.plan_id);
    assert_eq!(read.phases, plan.phases);
}

// Ensure invalid backup plans are rejected before writing.
#[test]
fn invalid_backup_plan_is_not_written() {
    let root = temp_dir("canic-backup-invalid-plan");
    let layout = BackupLayout::new(root.clone());
    let mut plan = valid_backup_plan();
    plan.plan_id.clear();

    let err = layout
        .write_backup_plan(&plan)
        .expect_err("invalid backup plan should fail");

    let plan_path = layout.backup_plan_path();
    fs::remove_dir_all(root).ok();
    assert!(matches!(err, PersistenceError::InvalidBackupPlan(_)));
    assert!(!plan_path.exists());
}

// Ensure execution journal writes create parent dirs and round-trip through validation.
#[test]
fn execution_journal_round_trips_through_layout() {
    let root = temp_dir("canic-backup-execution-journal-layout");
    let layout = BackupLayout::new(root.clone());
    let journal = valid_execution_journal();

    layout
        .write_execution_journal(&journal)
        .expect("write execution journal atomically");
    let read = layout
        .read_execution_journal()
        .expect("read execution journal");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(read.plan_id, journal.plan_id);
    assert_eq!(read.operations.len(), journal.operations.len());
}

// Ensure invalid execution journals are rejected before writing.
#[test]
fn invalid_execution_journal_is_not_written() {
    let root = temp_dir("canic-backup-invalid-execution-journal");
    let layout = BackupLayout::new(root.clone());
    let mut journal = valid_execution_journal();
    journal.plan_id.clear();

    let err = layout
        .write_execution_journal(&journal)
        .expect_err("invalid execution journal should fail");

    let journal_path = layout.execution_journal_path();
    fs::remove_dir_all(root).ok();
    assert!(matches!(err, PersistenceError::InvalidExecutionJournal(_)));
    assert!(!journal_path.exists());
}

// Ensure persisted plans and execution journals are checked together for resume.
#[test]
fn execution_integrity_verifies_plan_and_journal_match() {
    let root = temp_dir("canic-backup-execution-integrity");
    let layout = BackupLayout::new(root.clone());
    let plan = valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");

    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let report = layout
        .verify_execution_integrity()
        .expect("verify execution integrity");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(report.plan_id, plan.plan_id);
    assert!(report.verified);
    assert_eq!(report.plan_operations, plan.phases.len());
}

// Ensure resume cannot pair a plan with an unrelated execution journal.
#[test]
fn execution_integrity_rejects_plan_journal_operation_mismatch() {
    let root = temp_dir("canic-backup-execution-integrity-mismatch");
    let layout = BackupLayout::new(root.clone());
    let plan = valid_backup_plan();
    let mut journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    journal.operations[0].operation_id = "different-operation".to_string();

    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let err = layout
        .verify_execution_integrity()
        .expect_err("operation mismatch should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(
        err,
        PersistenceError::PlanJournalOperationMismatch { .. }
    ));
}

// Ensure invalid manifests are rejected before writing.
#[test]
fn invalid_manifest_is_not_written() {
    let root = temp_dir("canic-backup-invalid-manifest");
    let layout = BackupLayout::new(root.clone());
    let mut manifest = valid_manifest();
    manifest.fleet.discovery_topology_hash = "bad".to_string();

    let err = layout
        .write_manifest(&manifest)
        .expect_err("invalid manifest should fail");

    let manifest_path = layout.manifest_path();
    fs::remove_dir_all(root).ok();
    assert!(matches!(err, PersistenceError::InvalidManifest(_)));
    assert!(!manifest_path.exists());
}

// Ensure layout integrity verifies manifest, journal, and artifact checksums.
#[test]
fn integrity_verifies_durable_artifacts() {
    let root = temp_dir("canic-backup-integrity");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let journal = journal_with_checksum(checksum.hash.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let report = layout.verify_integrity().expect("verify integrity");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(report.backup_id, "fbk_test_001");
    assert!(report.verified);
    assert_eq!(report.manifest_members, 1);
    assert_eq!(report.durable_artifacts, 1);
    assert_eq!(report.artifacts[0].checksum, checksum.hash);
}

// Ensure mismatched manifest and journal backup IDs are rejected.
#[test]
fn integrity_rejects_backup_id_mismatch() {
    let root = temp_dir("canic-backup-integrity-id");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut journal = journal_with_checksum(checksum.hash);
    journal.backup_id = "other-backup".to_string();

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("backup id mismatch should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(err, PersistenceError::BackupIdMismatch { .. }));
}

// Ensure manifest and journal topology receipts cannot silently diverge.
#[test]
fn integrity_rejects_manifest_journal_topology_receipt_mismatch() {
    let root = temp_dir("canic-backup-integrity-topology");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut journal = journal_with_checksum(checksum.hash);
    journal.discovery_topology_hash =
        Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("topology receipt mismatch should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(
        err,
        PersistenceError::ManifestJournalTopologyReceiptMismatch { .. }
    ));
}

// Ensure incomplete journals cannot pass backup integrity verification.
#[test]
fn integrity_rejects_non_durable_artifacts() {
    let root = temp_dir("canic-backup-integrity-state");
    let layout = BackupLayout::new(root.clone());
    let mut journal = valid_journal();
    journal.artifacts[0].state = ArtifactState::Created;
    journal.artifacts[0].checksum = None;

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("non-durable artifact should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(err, PersistenceError::NonDurableArtifact { .. }));
}

// Ensure journals cannot include artifacts outside the manifest boundary.
#[test]
fn integrity_rejects_unexpected_journal_artifacts() {
    let root = temp_dir("canic-backup-integrity-extra");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut journal = journal_with_checksum(checksum.hash);
    let mut extra = journal.artifacts[0].clone();
    extra.snapshot_id = "extra-snapshot".to_string();
    journal.artifacts.push(extra);

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("unexpected journal artifact should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(
        err,
        PersistenceError::UnexpectedJournalArtifact { .. }
    ));
}

// Ensure manifest snapshot checksums cannot drift from the durable journal.
#[test]
fn integrity_rejects_manifest_journal_checksum_mismatch() {
    let root = temp_dir("canic-backup-integrity-manifest-checksum");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].source_snapshot.checksum =
        Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string());

    layout.write_manifest(&manifest).expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("manifest checksum mismatch should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(
        err,
        PersistenceError::ManifestJournalChecksumMismatch { .. }
    ));
}

// Ensure manifest and journal artifact paths cannot silently diverge.
#[test]
fn integrity_rejects_manifest_journal_artifact_path_mismatch() {
    let root = temp_dir("canic-backup-integrity-manifest-path");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].source_snapshot.artifact_path =
        "artifacts/different-root".to_string();

    layout.write_manifest(&manifest).expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");

    let err = layout
        .verify_integrity()
        .expect_err("manifest journal artifact path mismatch should fail");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(matches!(
        err,
        PersistenceError::ManifestJournalArtifactPathMismatch { .. }
    ));
}

// Build one valid manifest for persistence tests.
fn valid_manifest() -> FleetBackupManifest {
    FleetBackupManifest {
        manifest_version: 1,
        backup_id: "fbk_test_001".to_string(),
        created_at: "2026-04-10T12:00:00Z".to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "v1".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "single-root".to_string(),
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
            members: vec![FleetMember {
                role: "root".to_string(),
                canister_id: ROOT.to_string(),
                parent_canister_id: None,
                subnet_canister_id: Some(CHILD.to_string()),
                controller_hint: Some(ROOT.to_string()),
                identity_mode: IdentityMode::Fixed,
                verification_checks: vec![VerificationCheck {
                    kind: "status".to_string(),
                    roles: Vec::new(),
                }],
                source_snapshot: SourceSnapshot {
                    snapshot_id: "snap-root".to_string(),
                    module_hash: Some(HASH.to_string()),
                    code_version: Some("v0.30.0".to_string()),
                    artifact_path: "artifacts/root".to_string(),
                    checksum_algorithm: "sha256".to_string(),
                    checksum: None,
                },
            }],
        },
        verification: VerificationPlan {
            fleet_checks: Vec::new(),
            member_checks: Vec::new(),
        },
    }
}

// Build one valid execution journal for persistence tests.
fn valid_execution_journal() -> BackupExecutionJournal {
    let plan = valid_backup_plan();

    BackupExecutionJournal::from_plan(&plan).expect("execution journal")
}

// Build one valid backup plan for persistence tests.
fn valid_backup_plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
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
                module_hash: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: None,
            },
        ],
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Proven),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Proven,
        ),
        quiescence_policy: crate::plan::QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("backup plan")
}

// Build one valid durable journal for persistence tests.
fn valid_journal() -> DownloadJournal {
    journal_with_checksum(HASH.to_string())
}

// Build one durable journal with a caller-provided checksum.
fn journal_with_checksum(checksum: String) -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        discovery_topology_hash: Some(HASH.to_string()),
        pre_snapshot_topology_hash: Some(HASH.to_string()),
        operation_metrics: crate::journal::DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: ROOT.to_string(),
            snapshot_id: "snap-root".to_string(),
            state: ArtifactState::Durable,
            temp_path: None,
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(checksum),
            updated_at: "2026-04-10T12:00:00Z".to_string(),
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
