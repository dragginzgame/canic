use canic_backup::{
    execution::{BackupExecutionJournal, BackupExecutionResumeSummary},
    plan::{BackupPlan, BackupScopeKind},
};

pub(super) fn execution_layout_status(
    journal: &BackupExecutionJournal,
    has_manifest: bool,
) -> String {
    let summary = journal.resume_summary();
    if has_manifest && execution_is_complete(&summary) {
        "complete".to_string()
    } else if summary.failed_operations > 0 {
        "failed".to_string()
    } else if journal.preflight_accepted || summary.completed_operations > 0 {
        "running".to_string()
    } else {
        "dry-run".to_string()
    }
}

pub(super) const fn execution_is_complete(execution: &BackupExecutionResumeSummary) -> bool {
    execution.completed_operations + execution.skipped_operations == execution.total_operations
}

pub(super) fn backup_scope_label(plan: &BackupPlan) -> String {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootFleet => "non-root-deployment".to_string(),
        BackupScopeKind::Subtree => plan
            .selected_subtree_root
            .as_ref()
            .map_or_else(|| "subtree".to_string(), |root| format!("subtree:{root}")),
        BackupScopeKind::Member => plan
            .selected_subtree_root
            .as_ref()
            .map_or_else(|| "member".to_string(), |root| format!("member:{root}")),
        BackupScopeKind::MaintenanceRoot => "maintenance-root".to_string(),
    }
}
