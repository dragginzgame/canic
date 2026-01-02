//! Bootstrap workflows.
//!
//! This module contains **async orchestration logic only**.
//! It assumes the environment has already been initialized or restored
//! by lifecycle adapters.
//!
//! It must NOT:
//! - handle IC lifecycle hooks directly
//! - depend on init payload presence
//! - perform environment seeding or restoration
//! - import directory snapshots

pub mod nonroot;
pub mod root;

use crate::{Error, ThisError, workflow::WorkflowError};

///
/// BootstrapError
///

#[derive(Debug, ThisError)]
pub enum BootstrapError {
    #[error("missing required env fields: {0}")]
    MissingEnvFields(String),
}

impl From<BootstrapError> for Error {
    fn from(err: BootstrapError) -> Self {
        WorkflowError::from(err).into()
    }
}
