//! Module: backup::create::persistence
//!
//! Responsibility: persist and validate backup create execution layouts.
//! Does not own: backup planning, CLI option parsing, or runner execution.
//! Boundary: backup plan/journal filesystem state for create and resume.

use super::super::{BackupCommandError, layout::ensure_execution_journal_exists};
use canic_backup::{
    execution::BackupExecutionJournal, persistence::BackupLayout, plan::BackupPlan,
};
use std::path::Path;

pub(super) struct PersistedBackupCreateLayout {
    pub(super) plan: BackupPlan,
    pub(super) reused_existing: bool,
}

pub(super) fn persist_backup_create_layout(
    out: &Path,
    plan: &BackupPlan,
) -> Result<PersistedBackupCreateLayout, BackupCommandError> {
    let layout = BackupLayout::new(out.to_path_buf());
    if layout.backup_plan_path().is_file() {
        let existing = layout.read_backup_plan()?;
        ensure_resume_plan_compatible(&existing, plan)?;
        ensure_execution_journal_exists(&layout)?;
        layout.verify_execution_integrity()?;
        return Ok(PersistedBackupCreateLayout {
            plan: existing,
            reused_existing: true,
        });
    }

    let journal = BackupExecutionJournal::from_plan(plan)?;
    layout.write_backup_plan(plan)?;
    layout.write_execution_journal(&journal)?;
    layout.verify_execution_integrity()?;
    Ok(PersistedBackupCreateLayout {
        plan: plan.clone(),
        reused_existing: false,
    })
}

fn ensure_resume_plan_compatible(
    existing: &BackupPlan,
    requested: &BackupPlan,
) -> Result<(), BackupCommandError> {
    compare_resume_field("deployment", &existing.fleet, &requested.fleet)?;
    compare_resume_field("environment", &existing.environment, &requested.environment)?;
    compare_resume_field(
        "root_canister_id",
        &existing.root_canister_id,
        &requested.root_canister_id,
    )?;
    compare_resume_field(
        "selected_scope_kind",
        &format!("{:?}", existing.selected_scope_kind),
        &format!("{:?}", requested.selected_scope_kind),
    )?;
    compare_resume_field(
        "selected_subtree_root",
        &optional_string(existing.selected_subtree_root.as_ref()),
        &optional_string(requested.selected_subtree_root.as_ref()),
    )?;
    compare_resume_field(
        "requires_root_controller",
        &existing.requires_root_controller.to_string(),
        &requested.requires_root_controller.to_string(),
    )?;
    compare_resume_field(
        "snapshot_read_authority",
        &format!("{:?}", existing.snapshot_read_authority),
        &format!("{:?}", requested.snapshot_read_authority),
    )?;
    compare_resume_field(
        "quiescence_policy",
        &format!("{:?}", existing.quiescence_policy),
        &format!("{:?}", requested.quiescence_policy),
    )?;
    compare_resume_field(
        "targets",
        &target_signature(existing),
        &target_signature(requested),
    )?;
    compare_resume_field(
        "operations",
        &operation_signature(existing),
        &operation_signature(requested),
    )?;
    Ok(())
}

fn compare_resume_field(
    field: &'static str,
    existing: &str,
    requested: &str,
) -> Result<(), BackupCommandError> {
    if existing == requested {
        return Ok(());
    }

    Err(BackupCommandError::BackupLayoutMismatch {
        field,
        existing: existing.to_string(),
        requested: requested.to_string(),
    })
}

fn optional_string(value: Option<&String>) -> String {
    value.map_or_else(|| "-".to_string(), ToString::to_string)
}

fn target_signature(plan: &BackupPlan) -> String {
    plan.targets
        .iter()
        .map(|target| {
            format!(
                "{}:{}:{}:{}:{:?}:{:?}:{:?}:{}",
                target.canister_id,
                target.role.as_deref().unwrap_or("-"),
                target.parent_canister_id.as_deref().unwrap_or("-"),
                target.depth,
                target.control_authority,
                target.snapshot_read_authority,
                target.identity_mode,
                target.expected_module_hash.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn operation_signature(plan: &BackupPlan) -> String {
    plan.phases
        .iter()
        .map(|operation| {
            format!(
                "{}:{:?}:{}",
                operation.operation_id,
                operation.kind,
                operation.target_canister_id.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}
