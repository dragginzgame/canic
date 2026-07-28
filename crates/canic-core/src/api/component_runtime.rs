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
    },
    workflow::component_runtime,
};

///
/// ComponentRuntimeApi
///

pub struct ComponentRuntimeApi;

impl ComponentRuntimeApi {
    pub fn prepare_directory(
        request: ComponentRuntimeDirectoryPreparationRequest,
    ) -> Result<ComponentRuntimeStatusResponse, Error> {
        component_runtime::prepare_directory(request).map_err(Error::from)
    }

    pub fn status() -> Result<ComponentRuntimeStatusResponse, Error> {
        component_runtime::status().map_err(Error::from)
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
