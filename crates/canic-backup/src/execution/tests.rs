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

// Ensure new journals keep mutating work blocked until preflight evidence is accepted.
#[test]
fn journal_blocks_mutation_until_preflight_is_accepted() {
    let journal = journal();

    assert!(!journal.preflight_accepted);
    assert_eq!(
        journal.next_ready_operation().expect("next op").kind,
        BackupOperationKind::ValidateTopology
    );
    assert!(
        journal
            .operations
            .iter()
            .filter(|operation| operation.kind == BackupOperationKind::Stop)
            .all(|operation| operation.state == BackupExecutionOperationState::Blocked)
    );
}

// Ensure accepting the preflight bundle completes preflight gates and unblocks mutation.
#[test]
fn accepting_preflight_unblocks_first_mutating_operation() {
    let mut journal = journal();
    let plan = plan();
    let receipts = preflight_receipts(&plan);

    journal
        .accept_preflight_receipts_at(&receipts, Some("unix:10".to_string()))
        .expect("accept preflight");

    assert!(journal.preflight_accepted);
    assert_eq!(journal.preflight_id.as_deref(), Some(PREFLIGHT_ID));
    assert_eq!(
        journal.next_ready_operation().expect("next op").kind,
        BackupOperationKind::Stop
    );
    assert!(
        journal
            .operations
            .iter()
            .filter(|operation| operation.kind == BackupOperationKind::ValidateTopology)
            .all(|operation| operation.state == BackupExecutionOperationState::Completed)
    );
}

// Ensure typed preflight bundles must belong to the same plan as the journal.
#[test]
fn rejects_preflight_receipts_for_different_plan() {
    let mut journal = journal();
    let mut plan = plan();
    plan.plan_id = "plan-other".to_string();
    let receipts = preflight_receipts(&plan);

    let err = journal
        .accept_preflight_receipts_at(&receipts, Some("unix:10".to_string()))
        .expect_err("different plan rejects");

    assert!(matches!(
        err,
        BackupExecutionJournalError::PreflightPlanMismatch { expected, actual }
            if expected == "plan-001" && actual == "plan-other"
    ));
}

// Ensure operation execution advances in plan order.
#[test]
fn rejects_out_of_order_mutation() {
    let mut journal = accepted_journal();

    let err = journal
        .mark_operation_pending_at(5, Some("unix:20".to_string()))
        .expect_err("out-of-order operation rejects");

    assert!(matches!(
        err,
        BackupExecutionJournalError::OutOfOrderOperationTransition {
            requested: 5,
            next: 4
        }
    ));
}

// Ensure completed stop creates an explicit restart-required state.
#[test]
fn completed_stop_sets_restart_required() {
    let mut journal = accepted_journal();

    complete_operation(&mut journal, 4);

    assert!(journal.restart_required);
    let summary = journal.resume_summary();
    assert!(summary.restart_required);
    assert_eq!(
        summary.next_operation.expect("next op").kind,
        BackupOperationKind::CreateSnapshot
    );
}

// Ensure snapshot creation receipts must carry the created snapshot id.
#[test]
fn snapshot_completion_requires_snapshot_id() {
    let mut journal = accepted_journal();
    complete_operation(&mut journal, 4);
    journal
        .mark_operation_pending_at(5, Some("unix:30".to_string()))
        .expect("mark snapshot pending");
    let operation = journal.operations[5].clone();
    let receipt = BackupExecutionOperationReceipt::completed(
        &journal,
        &operation,
        Some("unix:31".to_string()),
    );
    let receipt_count = journal.operation_receipts.len();

    let err = journal
        .record_operation_receipt(receipt)
        .expect_err("missing snapshot id rejects");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "operation_receipts[].snapshot_id")
    );
    assert_eq!(journal.operation_receipts.len(), receipt_count);
    assert_eq!(
        journal.operations[5].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(
        journal.operations[5].state_updated_at.as_deref(),
        Some("unix:30")
    );
    assert!(journal.operations[5].blocking_reasons.is_empty());
}

// Ensure operation receipts always carry an audit timestamp.
#[test]
fn operation_receipts_require_updated_at() {
    let mut journal = accepted_journal();
    journal
        .mark_operation_pending_at(4, Some("unix:40".to_string()))
        .expect("mark stop pending");
    let operation = journal.operations[4].clone();
    let receipt = BackupExecutionOperationReceipt::completed(&journal, &operation, None);

    let err = journal
        .record_operation_receipt(receipt)
        .expect_err("missing receipt timestamp rejects");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "operation_receipts[].updated_at")
    );
    assert!(journal.operation_receipts.is_empty());
    assert_eq!(
        journal.operations[4].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(
        journal.operations[4].state_updated_at.as_deref(),
        Some("unix:40")
    );
    assert!(journal.operations[4].blocking_reasons.is_empty());
}

// Ensure a failed snapshot after stopping leaves restart-required visible and retryable.
#[test]
fn failed_snapshot_after_stop_is_retryable_and_requires_restart() {
    let mut journal = accepted_journal();
    complete_operation(&mut journal, 4);
    journal
        .mark_operation_pending_at(5, Some("unix:30".to_string()))
        .expect("mark snapshot pending");
    let operation = journal.operations[5].clone();
    let receipt = BackupExecutionOperationReceipt::failed(
        &journal,
        &operation,
        Some("unix:31".to_string()),
        "snapshot create failed".to_string(),
    );

    journal
        .record_operation_receipt(receipt)
        .expect("record snapshot failure");

    assert!(journal.restart_required);
    assert_eq!(
        journal.next_ready_operation().expect("next op").state,
        BackupExecutionOperationState::Failed
    );
    journal
        .retry_failed_operation_at(5, Some("unix:32".to_string()))
        .expect("retry failed snapshot");
    assert_eq!(
        journal.next_ready_operation().expect("next op").state,
        BackupExecutionOperationState::Ready
    );
}

// Ensure preflight identity cannot change once accepted.
#[test]
fn rejects_different_preflight_after_acceptance() {
    let mut journal = accepted_journal();

    let err = journal
        .accept_preflight_bundle_at("preflight-other".to_string(), Some("unix:11".to_string()))
        .expect_err("different preflight rejects");

    assert!(matches!(
        err,
        BackupExecutionJournalError::PreflightAlreadyAccepted { existing, attempted }
            if existing == PREFLIGHT_ID && attempted == "preflight-other"
    ));
}

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
