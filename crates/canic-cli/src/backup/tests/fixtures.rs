//! Module: backup::tests::fixtures
//!
//! Responsibility: shared backup CLI test fixtures and layout builders.
//! Does not own: production backup planning, persistence, or command dispatch.
//! Boundary: deterministic test data for backup command unit tests.

use super::super::*;
use crate::{support::path_stamp::backup_directory_stamp_to_unix, test_support::temp_dir};
use canic_backup::{
    artifacts::ArtifactChecksum,
    execution::{BackupExecutionJournal, BackupExecutionOperationReceipt},
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, DeploymentBackupManifest, DeploymentMember,
        DeploymentSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, BackupOperationKind, BackupPlan, BackupPlanBuildInput, BackupScopeKind,
        ControlAuthority, SnapshotReadAuthority, build_backup_plan,
    },
    registry::RegistryEntry,
};
use std::{fs, path::Path};

pub(super) const ROOT: &str = "aaaaa-aa";
pub(super) const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
pub(super) const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build one valid manifest for CLI verification tests.
pub(super) fn valid_manifest() -> DeploymentBackupManifest {
    valid_manifest_with("backup-test", "2026-05-03T00:00:00Z")
}

// Build one valid manifest with caller-provided summary fields.
pub(super) fn valid_manifest_with(backup_id: &str, created_at: &str) -> DeploymentBackupManifest {
    DeploymentBackupManifest {
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
                unit_id: "deployment".to_string(),
                kind: BackupUnitKind::Single,
                roles: vec!["root".to_string()],
            }],
        },
        deployment: DeploymentSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![deployment_member()],
        },
        verification: VerificationPlan::default(),
    }
}

// Build one valid manifest member.
fn deployment_member() -> DeploymentMember {
    DeploymentMember {
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
            code_version: Some("v0.30.3".to_string()),
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
        },
    }
}

// Build one backup plan for create dry-run persistence tests.
pub(super) fn valid_backup_plan() -> BackupPlan {
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
                module_hash: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: Some(HASH.to_string()),
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

// Build the executable counterpart of the standard dry-run backup plan.
pub(super) fn valid_executable_backup_plan() -> BackupPlan {
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
                module_hash: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: Some(HASH.to_string()),
            },
        ],
        control_authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
        snapshot_read_authority: SnapshotReadAuthority::operator_controller(
            AuthorityEvidence::Proven,
        ),
        quiescence_policy: canic_backup::plan::QuiescencePolicy::CrashConsistent,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("executable backup plan")
}

// Write a manifest plus matching plan but no execution journal.
pub(super) fn write_manifest_plan_without_execution_journal(root: &Path) {
    let layout = BackupLayout::new(root.to_path_buf());
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
}

// Write a manifest plus matching plan and caller-provided execution journal.
pub(super) fn write_manifest_plan_journal(root: &Path, journal: BackupExecutionJournal) {
    let layout = BackupLayout::new(root.to_path_buf());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
}

// Read backup status from one caller-provided execution journal layout.
pub(super) fn backup_status_for_execution_journal(
    name: &str,
    journal: BackupExecutionJournal,
    write_manifest: bool,
) -> BackupDryRunStatusReport {
    let root = temp_dir(name);
    let layout = BackupLayout::new(root.clone());
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    if write_manifest {
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
    }
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read backup status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::DryRun(report) = report else {
        panic!("expected execution status");
    };
    report
}

// Build an execution journal after the preflight gate has been accepted.
pub(super) fn accepted_execution_journal() -> BackupExecutionJournal {
    let mut journal =
        BackupExecutionJournal::from_plan(&valid_backup_plan()).expect("execution journal");
    journal
        .accept_preflight_bundle_at("preflight-test".to_string(), Some("unix:10".to_string()))
        .expect("accept preflight");
    journal
}

// Complete one operation in an execution journal with the fields required by that operation kind.
pub(super) fn complete_execution_operation(journal: &mut BackupExecutionJournal, sequence: usize) {
    journal
        .mark_operation_pending_at(sequence, Some(format!("unix:{sequence}0")))
        .expect("mark operation pending");
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation exists")
        .clone();
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        &operation,
        Some(format!("unix:{sequence}1")),
    );
    match operation.kind {
        BackupOperationKind::CreateSnapshot => {
            receipt.snapshot_id = Some("snap-app".to_string());
        }
        BackupOperationKind::DownloadSnapshot => {
            receipt.artifact_path = Some("artifacts/app".to_string());
        }
        BackupOperationKind::VerifyArtifact => {
            receipt.checksum = Some(HASH.to_string());
        }
        _ => {}
    }
    journal
        .record_operation_receipt(receipt)
        .expect("record completed operation");
}

// Fail one operation in an execution journal.
pub(super) fn fail_execution_operation(
    journal: &mut BackupExecutionJournal,
    sequence: usize,
    reason: &str,
) {
    journal
        .mark_operation_pending_at(sequence, Some(format!("unix:{sequence}0")))
        .expect("mark operation pending");
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation exists")
        .clone();
    let receipt = BackupExecutionOperationReceipt::failed(
        journal,
        &operation,
        Some(format!("unix:{sequence}1")),
        reason.to_string(),
    );
    journal
        .record_operation_receipt(receipt)
        .expect("record failed operation");
}

// Build one durable journal with a caller-provided checksum.
pub(super) fn journal_with_checksum(checksum: String) -> DownloadJournal {
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
pub(super) fn created_journal() -> DownloadJournal {
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
pub(super) fn write_artifact(root: &Path, bytes: &[u8]) -> ArtifactChecksum {
    let path = root.join("artifacts/root");
    fs::create_dir_all(path.parent().expect("artifact has parent")).expect("create artifacts");
    fs::write(&path, bytes).expect("write artifact");
    ArtifactChecksum::from_bytes(bytes)
}

pub(super) fn unix_marker_for_stamp(stamp: &str) -> String {
    format!(
        "unix:{}",
        backup_directory_stamp_to_unix(stamp).expect("valid backup directory stamp")
    )
}
