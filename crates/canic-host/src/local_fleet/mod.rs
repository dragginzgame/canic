//! Public developer Fleet ownership over a bounded, persistent PocketIC instance.
//!
//! The optional local-fleet feature does not participate in production canister builds.

#[cfg(test)]
mod tests;

pub mod model;
pub mod ops;
pub mod policy;
pub mod view;
pub mod workflow;

/// Local environment lifecycle failure; no stored PID is used as deletion authority.
#[derive(Debug, thiserror::Error)]
pub enum LocalFleetError {
    #[error("invalid local Fleet configuration or resource bound")]
    Configuration,
    #[error("local Fleet is already owned by another process")]
    Busy,
    #[error("retained local Fleet identity or configuration differs; explicit reset is required")]
    Identity,
    #[error("local Fleet operation requires its exact current session identity")]
    Session,
    #[error("local Fleet has no acknowledged checkpoint; its live state may be newer than disk")]
    UncleanCheckpoint,
    #[error("local Fleet checkpoint did not complete within the configured timeout")]
    CheckpointTimeout,
    #[error("local Fleet resource bound is exhausted")]
    Capacity,
    #[error("local Fleet state contains a symlink or unexpected file type")]
    UnsafePath,
    #[error("PocketIC local operation failed: {0}")]
    Platform(String),
    #[error("local Root rejected authority observation: {0}")]
    Root(canic_core::dto::error::Error),
    #[error(transparent)]
    Startup(#[from] ic_testkit::pic::PocketIcStartupError),
    #[error("PocketIC instance startup failed: {source}\nstdout: {stdout}\nstderr: {stderr}")]
    InstanceStartup {
        #[source]
        source: Box<ic_testkit::pic::PocketIcStartupError>,
        stdout: String,
        stderr: String,
    },
    #[error("local Fleet preparation rejected: {0}")]
    Preparation(String),
    #[error(transparent)]
    Ensure(
        Box<crate::fleet_ensure::EnsureWorkflowError<crate::fleet_ensure::IcpEnsurePlatformError>>,
    ),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
