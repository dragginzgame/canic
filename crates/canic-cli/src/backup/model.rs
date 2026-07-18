use canic_backup::journal::JournalResumeReport;
use serde::{Serialize, Serializer};
use std::path::PathBuf;

///
/// BackupCreateReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCreateReport {
    pub deployment: String,
    pub environment: String,
    pub out: PathBuf,
    pub plan_id: String,
    pub run_id: String,
    pub mode: BackupCreateMode,
    pub layout: BackupCreateLayout,
    pub status: BackupRunStatus,
    pub scope: String,
    pub targets: usize,
    pub operations: usize,
    pub executed_operations: usize,
}

///
/// BackupCreateMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupCreateMode {
    DryRun,
    Execute,
}

impl BackupCreateMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Execute => "execute",
        }
    }
}

///
/// BackupCreateLayout
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupCreateLayout {
    Existing,
    New,
}

impl BackupCreateLayout {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Existing => "existing",
            Self::New => "new",
        }
    }
}

///
/// BackupRunStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupRunStatus {
    Complete,
    Paused,
    Planned,
    Running,
}

impl BackupRunStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Paused => "paused",
            Self::Planned => "planned",
            Self::Running => "running",
        }
    }
}

///
/// BackupListEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupListEntry {
    pub dir: PathBuf,
    pub backup_id: String,
    pub created_at: String,
    pub members: usize,
    pub status: BackupListStatus,
}

///
/// BackupListStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupListStatus {
    Complete,
    DryRun,
    Failed,
    InvalidManifest,
    InvalidPlan,
    InvalidPlanJournal,
    Ok,
    Running,
}

impl BackupListStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::DryRun => "dry-run",
            Self::Failed => "failed",
            Self::InvalidManifest => "invalid-manifest",
            Self::InvalidPlan => "invalid-plan",
            Self::InvalidPlanJournal => "invalid-plan-journal",
            Self::Ok => "ok",
            Self::Running => "running",
        }
    }
}

///
/// BackupPruneReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPruneReport {
    pub dry_run: bool,
    pub scanned: usize,
    pub selected: usize,
    pub pruned: usize,
    pub entries: Vec<BackupPruneEntry>,
}

///
/// BackupPruneEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPruneEntry {
    pub index: usize,
    pub dir: PathBuf,
    pub backup_id: String,
    pub status: BackupListStatus,
    pub action: BackupPruneAction,
}

///
/// BackupPruneAction
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupPruneAction {
    Removed,
    WouldRemove,
}

impl BackupPruneAction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Removed => "removed",
            Self::WouldRemove => "would-remove",
        }
    }
}

///
/// BackupStatusReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupStatusReport {
    Download(JournalResumeReport),
    DryRun(BackupDryRunStatusReport),
}

///
/// BackupDryRunStatusReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupDryRunStatusReport {
    pub layout_status: BackupExecutionLayoutStatus,
    pub plan_id: String,
    pub run_id: String,
    pub deployment: String,
    pub environment: String,
    pub targets: usize,
    pub operations: usize,
    pub execution: canic_backup::execution::BackupExecutionResumeSummary,
}

///
/// BackupExecutionLayoutStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupExecutionLayoutStatus {
    Complete,
    DryRun,
    Failed,
    Running,
}

impl BackupExecutionLayoutStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::DryRun => "dry-run",
            Self::Failed => "failed",
            Self::Running => "running",
        }
    }
}

impl Serialize for BackupExecutionLayoutStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

///
/// BackupInspectReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectReport {
    pub layout_status: BackupExecutionLayoutStatus,
    pub plan_id: String,
    pub run_id: String,
    pub deployment: String,
    pub environment: String,
    pub scope: String,
    pub targets: Vec<BackupInspectTarget>,
    pub operations: Vec<BackupInspectOperation>,
    pub execution: canic_backup::execution::BackupExecutionResumeSummary,
}

///
/// BackupInspectTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectTarget {
    pub role: String,
    pub canister_id: String,
    pub parent_canister_id: String,
    pub expected_module_hash: String,
    pub depth: u32,
    pub control_authority: String,
    pub snapshot_read_authority: String,
}

///
/// BackupInspectOperation
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectOperation {
    pub sequence: usize,
    pub kind: String,
    pub target_canister_id: String,
    pub state: String,
    pub blocking_reasons: Vec<String>,
}
