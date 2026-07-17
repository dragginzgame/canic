//! Module: backup::error
//!
//! Responsibility: backup command error type and conversions.
//! Does not own: command dispatch or backup operation behavior.
//! Boundary: public error surface for `canic backup` and restore integration.

use super::manifest::ManifestCommandError;
use canic_backup::{
    discovery::DiscoveryError, execution::BackupExecutionJournalError,
    persistence::PersistenceError, plan::BackupPlanError, runner::BackupRunnerError,
};
use canic_host::{
    icp::IcpCommandError, icp_config::IcpConfigError,
    installed_deployment::InstalledDeploymentError, registry::RegistryParseError,
    replica_query::ReplicaQueryError,
};
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
    Manifest(#[from] ManifestCommandError),

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

    #[error(transparent)]
    InstalledDeployment(#[from] InstalledDeploymentError),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(#[source] ReplicaQueryError),

    #[error("failed to read canic deployment state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

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
