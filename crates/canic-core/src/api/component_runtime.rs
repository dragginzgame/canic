//! Module: api::component_runtime
//!
//! Responsibility: expose managed Component runtime Directory preparation workflows.
//! Does not own: validation, stable mutation, endpoint authorization, or activation.
//! Boundary: maps typed internal failures into Canic's public error contract.

use crate::{
    dto::{
        component_registry::{
            ComponentRuntimeDirectoryPreparationRequest, ComponentRuntimeDirectoryStatusResponse,
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
    ) -> Result<ComponentRuntimeDirectoryStatusResponse, Error> {
        component_runtime::prepare_directory(request).map_err(Error::from)
    }

    pub fn directory_status() -> Result<ComponentRuntimeDirectoryStatusResponse, Error> {
        component_runtime::directory_status().map_err(Error::from)
    }
}
