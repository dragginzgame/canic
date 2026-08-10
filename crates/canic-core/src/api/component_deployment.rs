//! Module: api::component_deployment
//!
//! Responsibility: expose the current runtime's protected Component deployment context.
//! Does not own: context validation, persistence, authorization, or deployment planning.
//! Boundary: application policy reads the exact value retained during managed initialization.

use crate::{
    InternalError,
    dto::{component_deployment::ProtectedComponentDeployment, error::Error},
    ids::FleetServiceId,
    ops::storage::{StorageOpsError, fleet_activation::FleetActivationOps},
};

///
/// ComponentDeploymentApi
///
/// Local read-only access to immutable Component deployment policy.
///

pub struct ComponentDeploymentApi;

impl ComponentDeploymentApi {
    /// Return this Component tree's protected deployment context.
    pub fn current() -> Result<ProtectedComponentDeployment, Error> {
        FleetActivationOps::component_deployment()
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)
            .map_err(Into::into)
    }

    /// Require this active Component tree to be the exact Authority for one Fleet service.
    pub fn require_service_authority(service: &FleetServiceId) -> Result<(), Error> {
        crate::workflow::component_runtime::require_service_authority(service).map_err(Into::into)
    }
}
