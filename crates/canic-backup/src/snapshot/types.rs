use crate::{
    artifacts::ArtifactChecksumError, discovery::DiscoveryError, journal::JournalValidationError,
    manifest::ManifestValidationError, persistence::PersistenceError, topology::TopologyHash,
};
use std::{
    error::Error as StdError,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub type SnapshotDriverError = Box<dyn StdError + Send + Sync + 'static>;

///
/// SnapshotArtifact
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotArtifact {
    pub canister_id: String,
    pub snapshot_id: String,
    pub path: PathBuf,
    pub checksum: String,
}

///
/// SnapshotLifecycleMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotLifecycleMode {
    StopBeforeSnapshot,
    StopAndResume,
}

impl SnapshotLifecycleMode {
    /// Build the lifecycle mode from the optional post-snapshot resume flag.
    #[must_use]
    pub const fn from_resume_flag(resume_after_snapshot: bool) -> Self {
        if resume_after_snapshot {
            Self::StopAndResume
        } else {
            Self::StopBeforeSnapshot
        }
    }

    /// Return whether snapshot capture should stop the canister first.
    #[must_use]
    pub const fn stop_before_snapshot(self) -> bool {
        true
    }

    /// Return whether snapshot capture should resume the canister afterward.
    #[must_use]
    pub const fn resume_after_snapshot(self) -> bool {
        matches!(self, Self::StopAndResume)
    }
}

///
/// SnapshotDownloadConfig
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadConfig {
    pub canister: String,
    pub out: PathBuf,
    pub root: Option<String>,
    pub include_children: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub lifecycle: SnapshotLifecycleMode,
    pub backup_id: String,
    pub created_at: String,
    pub tool_name: String,
    pub tool_version: String,
    pub environment: String,
}

///
/// SnapshotDownloadResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadResult {
    pub artifacts: Vec<SnapshotArtifact>,
    pub planned_commands: Vec<String>,
}

///
/// SnapshotDownloadError
///

#[derive(Debug, ThisError)]
pub enum SnapshotDownloadError {
    #[error("missing --root when using --include-children")]
    MissingRegistrySource,

    #[error("snapshot capture requires stopping each canister before snapshot create")]
    SnapshotRequiresStoppedCanister,

    #[error("snapshot driver failed: {0}")]
    Driver(#[source] SnapshotDriverError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Checksum(#[from] ArtifactChecksumError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Journal(#[from] JournalValidationError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Manifest(#[from] SnapshotManifestError),
}

///
/// SnapshotDriver
///

pub trait SnapshotDriver {
    /// Load the root registry JSON used to resolve child snapshot targets.
    fn registry_json(&mut self, root: &str) -> Result<String, SnapshotDriverError>;

    /// Create one canister snapshot and return its snapshot id.
    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError>;

    /// Stop one canister before snapshot creation.
    fn stop_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError>;

    /// Start one canister after snapshot capture.
    fn start_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError>;

    /// Download one snapshot into the supplied artifact directory.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError>;

    /// Render the planned create command for dry-run output.
    fn create_snapshot_command(&self, canister_id: &str) -> String;

    /// Render the planned stop command for dry-run output.
    fn stop_canister_command(&self, canister_id: &str) -> String;

    /// Render the planned start command for dry-run output.
    fn start_canister_command(&self, canister_id: &str) -> String;

    /// Render the planned download command for dry-run output.
    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String;
}

///
/// SnapshotManifestInput
///

pub struct SnapshotManifestInput<'a> {
    pub backup_id: String,
    pub created_at: String,
    pub tool_name: String,
    pub tool_version: String,
    pub environment: String,
    pub root_canister: String,
    pub selected_canister: String,
    pub include_children: bool,
    pub targets: &'a [crate::discovery::SnapshotTarget],
    pub artifacts: &'a [SnapshotArtifact],
    pub discovery_topology_hash: TopologyHash,
    pub pre_snapshot_topology_hash: TopologyHash,
}

///
/// SnapshotManifestError
///

#[derive(Debug, ThisError)]
pub enum SnapshotManifestError {
    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error(
        "topology changed before snapshot start: discovery={discovery}, pre_snapshot={pre_snapshot}"
    )]
    TopologyChanged {
        discovery: String,
        pre_snapshot: String,
    },

    #[error("missing snapshot artifact for canister {0}")]
    MissingArtifact(String),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),
}
