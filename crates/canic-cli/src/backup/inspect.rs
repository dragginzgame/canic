use super::{
    BackupCommandError, BackupInspectOperation, BackupInspectOptions, BackupInspectReport,
    BackupInspectTarget,
};
use crate::backup::{
    labels::{
        backup_scope_label, control_authority_source_label, execution_layout_status,
        format_authority, operation_kind_label, operation_state_label,
        snapshot_read_authority_source_label,
    },
    reference::resolve_backup_dir,
};
use canic_backup::{
    execution::BackupExecutionJournalOperation, persistence::BackupLayout, plan::BackupTarget,
};

pub(super) fn backup_inspect(
    options: &BackupInspectOptions,
) -> Result<BackupInspectReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    let plan = layout.read_backup_plan()?;
    let journal = layout.read_execution_journal()?;
    layout.verify_execution_integrity()?;

    Ok(BackupInspectReport {
        layout_status: execution_layout_status(&journal, layout.manifest_path().is_file()),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        fleet: plan.fleet.clone(),
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
