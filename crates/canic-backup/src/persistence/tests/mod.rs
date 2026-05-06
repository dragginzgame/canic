use super::*;
use crate::{
    journal::{ArtifactJournalEntry, ArtifactState},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember, FleetSection,
        IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    },
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

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

// Ensure inspection reports manifest and journal agreement without artifact IO.
#[test]
fn inspect_reports_ready_layout_metadata() {
    let root = temp_dir("canic-backup-inspect-ready");
    let layout = BackupLayout::new(root.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&valid_journal())
        .expect("write journal");

    let report = layout.inspect().expect("inspect layout");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(report.backup_id, "fbk_test_001");
    assert!(report.backup_id_matches);
    assert!(report.journal_complete);
    assert!(report.ready_for_verify);
    assert_eq!(report.manifest_members, 1);
    assert_eq!(report.journal_artifacts, 1);
    assert_eq!(report.matched_artifacts, 1);
    assert!(report.topology_receipt_mismatches.is_empty());
    assert!(report.missing_journal_artifacts.is_empty());
    assert!(report.unexpected_journal_artifacts.is_empty());
    assert!(report.path_mismatches.is_empty());
    assert!(report.checksum_mismatches.is_empty());
}

// Ensure inspection surfaces path and checksum drift before full verification.
#[test]
fn inspect_reports_manifest_journal_provenance_drift() {
    let root = temp_dir("canic-backup-inspect-drift");
    let layout = BackupLayout::new(root.clone());
    let mut manifest = valid_manifest();
    manifest.fleet.members[0].source_snapshot.artifact_path = "artifacts/manifest-root".to_string();
    manifest.fleet.members[0].source_snapshot.checksum = Some(HASH.to_string());
    let mut journal = journal_with_checksum(
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
    );
    journal.artifacts[0].artifact_path = "artifacts/journal-root".to_string();
    journal.pre_snapshot_topology_hash =
        Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string());

    layout.write_manifest(&manifest).expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let report = layout.inspect().expect("inspect layout");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(!report.ready_for_verify);
    assert_eq!(report.matched_artifacts, 1);
    assert_eq!(report.topology_receipt_mismatches.len(), 1);
    assert_eq!(report.path_mismatches.len(), 1);
    assert_eq!(report.checksum_mismatches.len(), 1);
}

// Ensure inspection reports missing and unexpected artifact boundaries.
#[test]
fn inspect_reports_missing_and_unexpected_artifacts() {
    let root = temp_dir("canic-backup-inspect-boundary");
    let layout = BackupLayout::new(root.clone());
    let mut journal = valid_journal();
    journal.artifacts[0].snapshot_id = "other-snapshot".to_string();

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout.write_journal(&journal).expect("write journal");

    let report = layout.inspect().expect("inspect layout");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert!(!report.ready_for_verify);
    assert_eq!(report.matched_artifacts, 0);
    assert_eq!(report.missing_journal_artifacts.len(), 1);
    assert_eq!(report.unexpected_journal_artifacts.len(), 1);
}

// Ensure provenance reports source, topology, unit, and snapshot metadata.
#[test]
fn provenance_reports_manifest_and_journal_receipts() {
    let root = temp_dir("canic-backup-provenance");
    let layout = BackupLayout::new(root.clone());

    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&valid_journal())
        .expect("write journal");

    let report = layout.provenance().expect("read provenance");

    fs::remove_dir_all(root).expect("remove temp layout");
    assert_eq!(report.backup_id, "fbk_test_001");
    assert_eq!(report.manifest_backup_id, "fbk_test_001");
    assert_eq!(report.journal_backup_id, "fbk_test_001");
    assert!(report.backup_id_matches);
    assert_eq!(report.source_environment, "local");
    assert_eq!(report.source_root_canister, ROOT);
    assert_eq!(report.discovery_topology_hash, HASH);
    assert_eq!(
        report.journal_discovery_topology_hash,
        Some(HASH.to_string())
    );
    assert!(report.topology_receipts_match);
    assert!(report.topology_receipt_mismatches.is_empty());
    assert_eq!(report.backup_unit_count, 1);
    assert_eq!(report.member_count, 1);
    assert_eq!(report.consistency_mode, "crash-consistent");
    assert_eq!(report.backup_units[0].kind, "whole-fleet");
    assert_eq!(report.members[0].canister_id, ROOT);
    assert_eq!(report.members[0].identity_mode, "fixed");
    assert_eq!(report.members[0].module_hash, Some(HASH.to_string()));
    assert_eq!(report.members[0].wasm_hash, Some(HASH.to_string()));
    assert_eq!(report.members[0].journal_state, Some("Durable".to_string()));
    assert_eq!(report.members[0].journal_checksum, Some(HASH.to_string()));
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
            mode: ConsistencyMode::CrashConsistent,
            backup_units: vec![BackupUnit {
                unit_id: "whole-fleet".to_string(),
                kind: BackupUnitKind::WholeFleet,
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
            members: vec![FleetMember {
                role: "root".to_string(),
                canister_id: ROOT.to_string(),
                parent_canister_id: None,
                subnet_canister_id: Some(CHILD.to_string()),
                controller_hint: Some(ROOT.to_string()),
                identity_mode: IdentityMode::Fixed,
                restore_group: 1,
                verification_class: "basic".to_string(),
                verification_checks: vec![VerificationCheck {
                    kind: "call".to_string(),
                    method: Some("canic_ready".to_string()),
                    roles: Vec::new(),
                }],
                source_snapshot: SourceSnapshot {
                    snapshot_id: "snap-root".to_string(),
                    module_hash: Some(HASH.to_string()),
                    wasm_hash: Some(HASH.to_string()),
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

// Build a unique temporary layout directory.
fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
