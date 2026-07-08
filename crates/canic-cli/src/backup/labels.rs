use canic_backup::{
    execution::{BackupExecutionJournal, BackupExecutionResumeSummary},
    plan::{BackupPlan, BackupScopeKind},
};

use crate::backup::BackupExecutionLayoutStatus;

pub(super) fn execution_layout_status(
    journal: &BackupExecutionJournal,
    has_manifest: bool,
) -> BackupExecutionLayoutStatus {
    let summary = journal.resume_summary();
    if has_manifest && execution_is_complete(&summary) {
        BackupExecutionLayoutStatus::Complete
    } else if summary.failed_operations > 0 {
        BackupExecutionLayoutStatus::Failed
    } else if journal.preflight_accepted || summary.completed_operations > 0 {
        BackupExecutionLayoutStatus::Running
    } else {
        BackupExecutionLayoutStatus::DryRun
    }
}

pub(super) const fn execution_is_complete(execution: &BackupExecutionResumeSummary) -> bool {
    execution.completed_operations + execution.skipped_operations == execution.total_operations
}

pub(super) fn backup_scope_label(plan: &BackupPlan) -> String {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootDeployment => "non-root-deployment".to_string(),
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
