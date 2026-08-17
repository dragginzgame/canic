//! Module: api::component_runtime
//!
//! Responsibility: expose managed Component Directory preparation and runtime activation.
//! Does not own: validation, stable mutation, endpoint authorization, or root distribution.
//! Boundary: maps typed internal failures into Canic's public error contract.

use crate::{
    dto::{
        component_registry::{
            ComponentRuntimeActivationRequest, ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimeStatusResponse,
        },
        error::Error,
        role::ComponentRuntimeOperationStatus,
    },
    workflow::{component_runtime, runtime::fleet_activation::FleetActivationWorkflow},
};

///
/// ComponentRuntimeApi
///

pub struct ComponentRuntimeApi;

impl ComponentRuntimeApi {
    /// Converge one managed runtime through its single role-owned command.
    pub fn configure(
        request: ComponentRuntimeDirectoryPreparationRequest,
    ) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, Error> {
        component_runtime::configure(request).map_err(Error::from)
    }

    pub fn prepare_directory(
        request: ComponentRuntimeDirectoryPreparationRequest,
    ) -> Result<ComponentRuntimeStatusResponse, Error> {
        component_runtime::prepare_directory(request).map_err(Error::from)
    }

    pub fn status() -> Result<ComponentRuntimeStatusResponse, Error> {
        component_runtime::status().map_err(Error::from)
    }

    /// Return the complete target-local runtime configuration operation projection.
    pub fn operation_status(
        operation_id: [u8; 32],
    ) -> Result<ComponentRuntimeOperationStatus, Error> {
        let fleet_activation = FleetActivationWorkflow::status().map_err(Error::from)?;
        let runtime = component_runtime::status().map_err(Error::from)?;
        if fleet_activation.identity.operation_id != operation_id
            || runtime.operation_id != operation_id
        {
            return Err(Error::from_registered(
                crate::diagnostics::codes::STATE_CONFLICT,
            ));
        }

        Ok(ComponentRuntimeOperationStatus {
            operation_id,
            fleet_activation,
            runtime,
        })
    }

    pub fn synchronize_directory(
        request: ComponentRuntimeDirectorySynchronizationRequest,
    ) -> Result<ComponentRuntimeStatusResponse, Error> {
        component_runtime::synchronize_directory(request).map_err(Error::from)
    }

    pub fn activate(
        request: ComponentRuntimeActivationRequest,
    ) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, Error> {
        component_runtime::activate(request).map_err(Error::from)
    }
}
