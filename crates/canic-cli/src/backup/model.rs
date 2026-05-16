use canic_backup::journal::JournalResumeReport;
use serde::Serialize;
use std::path::PathBuf;

///
/// BackupCreateReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCreateReport {
    pub fleet: String,
    pub network: String,
    pub out: PathBuf,
    pub plan_id: String,
    pub run_id: String,
    pub mode: String,
    pub layout: String,
    pub status: String,
    pub scope: String,
    pub targets: usize,
    pub operations: usize,
    pub executed_operations: usize,
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
    pub status: String,
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
    pub status: String,
    pub action: String,
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
    pub layout_status: String,
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub targets: usize,
    pub operations: usize,
    pub execution: canic_backup::execution::BackupExecutionResumeSummary,
}

///
/// BackupInspectReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectReport {
    pub layout_status: String,
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
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
