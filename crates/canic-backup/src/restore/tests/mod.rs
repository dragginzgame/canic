use super::*;
use crate::artifacts::ArtifactChecksum;
use crate::manifest::{
    BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember, FleetSection,
    IdentityMode, MemberVerificationChecks, SourceMetadata, SourceSnapshot, ToolMetadata,
    VerificationCheck, VerificationPlan,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const CHILD_TWO: &str = "r7inp-6aaaa-aaaaa-aaabq-cai";
const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build a one-operation ready journal for command preview tests.
fn command_preview_journal(
    operation: RestoreApplyOperationKind,
    verification_kind: Option<&str>,
) -> RestoreApplyJournal {
    let mut journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some("/tmp/canic-backup-restore".to_string()),
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::default(),
        pending_operations: 0,
        ready_operations: 1,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![RestoreApplyJournalOperation {
            sequence: 0,
            operation,
            state: RestoreApplyOperationState::Ready,
            state_updated_at: None,
            blocking_reasons: Vec::new(),
            member_order: 0,
            source_canister: ROOT.to_string(),
            target_canister: ROOT.to_string(),
            role: "root".to_string(),
            snapshot_id: Some("snap-root".to_string()),
            artifact_path: Some("artifacts/root".to_string()),
            verification_kind: verification_kind.map(str::to_string),
        }],
        operation_receipts: Vec::new(),
    };
    journal.operation_counts =
        RestoreApplyOperationKindCounts::from_operations(&journal.operations);

    journal.validate().expect("journal should validate");
    journal
}

// Build one valid manifest with a parent and child that restore in topology order.
fn valid_manifest(identity_mode: IdentityMode) -> FleetBackupManifest {
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
                unit_id: "subtree".to_string(),
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
                fleet_member("app", CHILD, Some(ROOT), identity_mode),
                fleet_member("root", ROOT, None, IdentityMode::Fixed),
            ],
        },
        verification: VerificationPlan {
            fleet_checks: Vec::new(),
            member_checks: Vec::new(),
        },
    }
}

// Build one manifest member for restore planning tests.
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
        subnet_canister_id: None,
        controller_hint: Some(ROOT.to_string()),
        identity_mode,
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: Vec::new(),
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: format!("snap-{role}"),
            module_hash: Some(HASH.to_string()),
            wasm_hash: Some(HASH.to_string()),
            code_version: Some("v0.30.0".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}
// Write one artifact and record its path and checksum in the test manifest.
fn set_member_artifact(
    manifest: &mut FleetBackupManifest,
    canister_id: &str,
    root: &Path,
    artifact_path: &str,
    bytes: &[u8],
) {
    let full_path = root.join(artifact_path);
    fs::create_dir_all(full_path.parent().expect("artifact parent")).expect("create parent");
    fs::write(&full_path, bytes).expect("write artifact");
    let checksum = ArtifactChecksum::from_bytes(bytes);
    let member = manifest
        .fleet
        .members
        .iter_mut()
        .find(|member| member.canister_id == canister_id)
        .expect("member should exist");
    member.source_snapshot.artifact_path = artifact_path.to_string();
    member.source_snapshot.checksum = Some(checksum.hash);
}

// Return a unique temporary directory for restore tests.
fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("{name}-{nanos}"))
}

mod apply_dry_run;
mod apply_journal;
mod plan;
