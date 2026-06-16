//! Module: backup::error
//!
//! Responsibility: backup command error type and conversions.
//! Does not own: command dispatch or backup operation behavior.
//! Boundary: public error surface for `canic backup` and restore integration.

use canic_backup::{
    discovery::DiscoveryError, execution::BackupExecutionJournalError,
    persistence::PersistenceError, plan::BackupPlanError, runner::BackupRunnerError,
};
use canic_host::registry::RegistryParseError;
use thiserror::Error as ThisError;

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error("backup plan {plan_id} is a dry-run layout, not a complete backup")]
    DryRunNotComplete { plan_id: String },

    #[error("backup reference {reference} was not found under backups; run `canic backup list`")]
    BackupReferenceNotFound { reference: String },

    #[error("backup reference {reference} is ambiguous under backups; use `--dir <dir>`")]
    BackupReferenceAmbiguous { reference: String },

    #[error("manifest: {0}")]
    Manifest(String),

    #[error(
        "backup layout at --out is for a different request: {field} existing={existing}, requested={requested}"
    )]
    BackupLayoutMismatch {
        field: &'static str,
        existing: String,
        requested: String,
    },

    #[error("backup layout is incomplete: missing {missing}")]
    BackupLayoutIncomplete { missing: &'static str },

    #[error(
        "deployment target {deployment} is not installed on network {network}; run `canic install <fleet-template>` or `canic deploy register {deployment} --fleet-template <fleet-template> --root <principal> --allow-unverified` before planning a backup"
    )]
    NoInstalledDeployment { network: String, deployment: String },

    #[error(
        "deployment target {deployment} points to root {root}, but that canister is not present on local network {network}. Local ICP CLI replica state is not persistent; run `canic install <fleet-template>` to recreate it or re-register {deployment} with `canic deploy register {deployment} --fleet-template <fleet-template> --root <principal> --allow-unverified`."
    )]
    LostLocalDeployment {
        network: String,
        deployment: String,
        root: String,
    },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("registry entry {canister_id} is not a valid principal")]
    InvalidRegistryPrincipal { canister_id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    BackupPlan(#[from] BackupPlanError),

    #[error(transparent)]
    BackupExecutionJournal(#[from] BackupExecutionJournalError),

    #[error(transparent)]
    BackupRunner(#[from] BackupRunnerError),
}
