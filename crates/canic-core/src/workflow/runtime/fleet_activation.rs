//! Module: workflow::runtime::fleet_activation
//!
//! Responsibility: expose the canonical protected Fleet activation status.
//! Does not own: stable conversion, endpoint authorization, or activation mutation.
//! Boundary: the runtime role selects root-only projection before ops validates the record.

use crate::{
    InternalError,
    dto::fleet_activation::FleetActivationStatusResponse,
    ops::{
        runtime::env::EnvOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

///
/// FleetActivationWorkflow
///

pub struct FleetActivationWorkflow;

impl FleetActivationWorkflow {
    pub fn status() -> Result<FleetActivationStatusResponse, InternalError> {
        FleetActivationOps::status(EnvOps::is_root())
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }
}
