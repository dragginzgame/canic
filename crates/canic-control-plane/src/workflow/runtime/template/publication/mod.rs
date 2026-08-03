mod cost_guard;
mod error;
mod lifecycle;
mod release;
mod snapshot;
mod store;

use crate::{
    dto::template::{WasmStoreAdminCommand, WasmStoreAdminResponse},
    ops::component_registry::ComponentRegistryOps,
};
use canic_core::control_plane_support::error::InternalError;

use self::cost_guard::{PUBLICATION_ADMIN_COMMAND_KIND, PublicationCostGuard};

///
/// WasmStorePublicationWorkflow
///

pub struct WasmStorePublicationWorkflow;

impl WasmStorePublicationWorkflow {
    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn handle_admin(
        cmd: WasmStoreAdminCommand,
    ) -> Result<WasmStoreAdminResponse, InternalError> {
        ComponentRegistryOps::require_root_store_admin_open()?;
        match cmd {
            WasmStoreAdminCommand::PublishActiveReleaseSet => {
                let cost_guard = PublicationCostGuard::reserve(PUBLICATION_ADMIN_COMMAND_KIND)?;
                let result = Self::publish_active_release_set_to_adopted_store(cost_guard.permit())
                    .await
                    .map(|()| WasmStoreAdminResponse::PublishedActiveReleaseSet);
                cost_guard.settle(result)
            }
        }
    }
}
