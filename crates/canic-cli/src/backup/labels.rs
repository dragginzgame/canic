use canic_backup::{
    execution::{
        BackupExecutionJournal, BackupExecutionOperationState, BackupExecutionResumeSummary,
    },
    plan::{
        AuthorityEvidence, BackupOperationKind, BackupPlan, BackupScopeKind,
        ControlAuthoritySource, SnapshotReadAuthoritySource,
    },
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
        BackupScopeKind::NonRootFleet => "non-root-fleet".to_string(),
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

pub(super) fn format_authority(source: &str, evidence: &AuthorityEvidence) -> String {
    format!("{source}/{}", authority_evidence_label(evidence))
}

pub(super) const fn control_authority_source_label(source: &ControlAuthoritySource) -> &str {
    match source {
        ControlAuthoritySource::Unknown => "unknown",
        ControlAuthoritySource::RootController => "root-controller",
        ControlAuthoritySource::OperatorController => "operator-controller",
        ControlAuthoritySource::AlternateController { .. } => "alternate-controller",
    }
}

pub(super) const fn snapshot_read_authority_source_label(
    source: &SnapshotReadAuthoritySource,
) -> &str {
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

pub(super) const fn operation_kind_label(kind: &BackupOperationKind) -> &str {
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

pub(super) const fn operation_state_label(state: &BackupExecutionOperationState) -> &str {
    match state {
        BackupExecutionOperationState::Ready => "ready",
        BackupExecutionOperationState::Pending => "pending",
        BackupExecutionOperationState::Blocked => "blocked",
        BackupExecutionOperationState::Completed => "completed",
        BackupExecutionOperationState::Failed => "failed",
        BackupExecutionOperationState::Skipped => "skipped",
    }
}
