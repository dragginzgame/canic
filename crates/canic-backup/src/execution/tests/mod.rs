//! Module: execution::tests
//!
//! Responsibility: shared execution journal test fixtures.
//! Does not own: production execution journal logic.
//! Boundary: fixtures for execution journal unit tests.

mod preflight;
mod receipts;
mod restart;
mod transitions;

use super::*;
use crate::{
    manifest::IdentityMode,
    plan::{
        AuthorityEvidence, BackupExecutionPreflightReceipts, BackupPlanBuildInput, BackupScopeKind,
        ControlAuthority, QuiescencePreflightReceipt, QuiescencePreflightTarget,
        SnapshotReadAuthority, TopologyPreflightReceipt, TopologyPreflightTarget,
        build_backup_plan,
    },
    registry::RegistryEntry,
};

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const PREFLIGHT_ID: &str = "preflight-001";

fn journal() -> BackupExecutionJournal {
    BackupExecutionJournal::from_plan(&plan()).expect("execution journal")
}

fn accepted_journal() -> BackupExecutionJournal {
    let mut journal = journal();
    journal
        .accept_preflight_bundle_at(PREFLIGHT_ID.to_string(), Some("unix:10".to_string()))
        .expect("accept preflight");
    journal
}

fn complete_operation(journal: &mut BackupExecutionJournal, sequence: usize) {
    journal
        .mark_operation_pending_at(sequence, Some(format!("unix:{sequence}0")))
        .expect("mark pending");
    let operation = journal.operations[sequence].clone();
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        &operation,
        Some(format!("unix:{sequence}1")),
    );
    if operation.kind == BackupOperationKind::CreateSnapshot {
        receipt.snapshot_id = Some("0000000000000001ffffffffffc000020101".to_string());
    }
    if operation.kind == BackupOperationKind::DownloadSnapshot {
        receipt.artifact_path = Some("backups/demo/app.snapshot".to_string());
    }
    if operation.kind == BackupOperationKind::VerifyArtifact {
        receipt.checksum =
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string());
    }
    journal
        .record_operation_receipt(receipt)
        .expect("record receipt");
}

fn plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-001".to_string(),
        run_id: "run-001".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        registry: &registry(),
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Proven),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Proven,
        ),
        quiescence_policy: crate::plan::QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("backup plan")
}

fn preflight_receipts(plan: &BackupPlan) -> BackupExecutionPreflightReceipts {
    let targets = plan
        .targets
        .iter()
        .map(TopologyPreflightTarget::from)
        .collect::<Vec<_>>();
    let quiescence_targets = plan
        .targets
        .iter()
        .map(QuiescencePreflightTarget::from)
        .collect::<Vec<_>>();
    BackupExecutionPreflightReceipts {
        plan_id: plan.plan_id.clone(),
        preflight_id: PREFLIGHT_ID.to_string(),
        validated_at: "unix:10".to_string(),
        expires_at: "unix:20".to_string(),
        topology: TopologyPreflightReceipt {
            plan_id: plan.plan_id.clone(),
            preflight_id: PREFLIGHT_ID.to_string(),
            topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
            topology_hash_at_preflight: plan.topology_hash_before_quiesce.clone(),
            targets,
            validated_at: "unix:10".to_string(),
            expires_at: "unix:20".to_string(),
            message: None,
        },
        control_authority: Vec::new(),
        snapshot_read_authority: Vec::new(),
        quiescence: QuiescencePreflightReceipt {
            plan_id: plan.plan_id.clone(),
            preflight_id: PREFLIGHT_ID.to_string(),
            quiescence_policy: plan.quiescence_policy.clone(),
            accepted: true,
            targets: quiescence_targets,
            validated_at: "unix:10".to_string(),
            expires_at: "unix:20".to_string(),
            message: None,
        },
    }
}

fn registry() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            pid: ROOT.to_string(),
            role: Some("root".to_string()),
            kind: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: APP.to_string(),
            role: Some("app".to_string()),
            kind: Some("singleton".to_string()),
            parent_pid: Some(ROOT.to_string()),
            module_hash: None,
        },
    ]
}
