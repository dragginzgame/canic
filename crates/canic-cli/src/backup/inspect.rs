use super::{
    BackupCommandError, BackupInspectOperation, BackupInspectOptions, BackupInspectReport,
    BackupInspectTarget,
};
use crate::backup::{
    labels::{backup_scope_label, execution_layout_status},
    layout::ensure_execution_journal_exists,
    reference::resolve_backup_dir,
};
use canic_backup::{
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, BackupOperationKind, BackupTarget, ControlAuthoritySource,
        SnapshotReadAuthoritySource,
    },
};

pub(super) fn backup_inspect(
    options: &BackupInspectOptions,
) -> Result<BackupInspectReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    let plan = layout.read_backup_plan()?;
    ensure_execution_journal_exists(&layout)?;
    let journal = layout.read_execution_journal()?;
    layout.verify_execution_integrity()?;

    Ok(BackupInspectReport {
        layout_status: execution_layout_status(&journal, layout.manifest_path().is_file()),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        deployment: plan.fleet.clone(),
        network: plan.network.clone(),
        scope: backup_scope_label(&plan),
        targets: plan.targets.iter().map(inspect_target).collect(),
        operations: journal.operations.iter().map(inspect_operation).collect(),
        execution: journal.resume_summary(),
    })
}

fn inspect_target(target: &BackupTarget) -> BackupInspectTarget {
    BackupInspectTarget {
        role: target.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target
            .parent_canister_id
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        expected_module_hash: target
            .expected_module_hash
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        depth: target.depth,
        control_authority: format_authority(
            control_authority_source_label(&target.control_authority.source),
            &target.control_authority.evidence,
        ),
        snapshot_read_authority: format_authority(
            snapshot_read_authority_source_label(&target.snapshot_read_authority.source),
            &target.snapshot_read_authority.evidence,
        ),
    }
}

fn inspect_operation(operation: &BackupExecutionJournalOperation) -> BackupInspectOperation {
    BackupInspectOperation {
        sequence: operation.sequence,
        kind: operation_kind_label(&operation.kind).to_string(),
        target_canister_id: operation
            .target_canister_id
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        state: operation_state_label(&operation.state).to_string(),
        blocking_reasons: operation.blocking_reasons.clone(),
    }
}

fn format_authority(source: &str, evidence: &AuthorityEvidence) -> String {
    format!("{source}/{}", authority_evidence_label(evidence))
}

const fn control_authority_source_label(source: &ControlAuthoritySource) -> &str {
    match source {
        ControlAuthoritySource::Unknown => "unknown",
        ControlAuthoritySource::RootController => "root-controller",
        ControlAuthoritySource::OperatorController => "operator-controller",
        ControlAuthoritySource::AlternateController { .. } => "alternate-controller",
    }
}

const fn snapshot_read_authority_source_label(source: &SnapshotReadAuthoritySource) -> &str {
    match source {
        SnapshotReadAuthoritySource::Unknown => "unknown",
        SnapshotReadAuthoritySource::OperatorController => "operator-controller",
        SnapshotReadAuthoritySource::SnapshotVisibility => "snapshot-visibility",
        SnapshotReadAuthoritySource::RootConfiguredRead => "root-configured-read",
        SnapshotReadAuthoritySource::RootMediatedTransfer => "root-mediated-transfer",
    }
}

const fn authority_evidence_label(evidence: &AuthorityEvidence) -> &str {
    match evidence {
        AuthorityEvidence::Proven => "proven",
        AuthorityEvidence::Declared => "declared",
        AuthorityEvidence::Unknown => "unknown",
    }
}

const fn operation_kind_label(kind: &BackupOperationKind) -> &str {
    match kind {
        BackupOperationKind::ValidateTopology => "validate-topology",
        BackupOperationKind::ValidateControlAuthority => "validate-control-authority",
        BackupOperationKind::ValidateSnapshotReadAuthority => "validate-snapshot-read-authority",
        BackupOperationKind::ValidateQuiescencePolicy => "validate-quiescence-policy",
        BackupOperationKind::Stop => "stop",
        BackupOperationKind::CreateSnapshot => "create-snapshot",
        BackupOperationKind::Start => "start",
        BackupOperationKind::DownloadSnapshot => "download-snapshot",
        BackupOperationKind::VerifyArtifact => "verify-artifact",
        BackupOperationKind::FinalizeManifest => "finalize-manifest",
    }
}

const fn operation_state_label(state: &BackupExecutionOperationState) -> &str {
    match state {
        BackupExecutionOperationState::Ready => "ready",
        BackupExecutionOperationState::Pending => "pending",
        BackupExecutionOperationState::Blocked => "blocked",
        BackupExecutionOperationState::Completed => "completed",
        BackupExecutionOperationState::Failed => "failed",
        BackupExecutionOperationState::Skipped => "skipped",
    }
}
