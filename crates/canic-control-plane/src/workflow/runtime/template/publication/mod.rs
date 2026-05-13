mod fleet;
mod lifecycle;
mod release;
mod store;

use crate::{
    dto::template::{
        WasmStoreAdminCommand, WasmStoreAdminResponse, WasmStoreFinalizedStoreResponse,
    },
    ids::CanisterRole,
};
use canic_core::__control_plane_core as cp_core;
use cp_core::InternalError;

const WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

///
/// WasmStorePublicationWorkflow
///

pub struct WasmStorePublicationWorkflow;

impl WasmStorePublicationWorkflow {
    const WASM_STORE_CAPACITY_EXCEEDED_MESSAGE: &str = "wasm store capacity exceeded";

    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn handle_admin(
        cmd: WasmStoreAdminCommand,
    ) -> Result<WasmStoreAdminResponse, InternalError> {
        match cmd {
            WasmStoreAdminCommand::PublishCurrentReleaseToStore { store_pid } => {
                Self::publish_current_release_set_to_store(store_pid).await?;
                Ok(WasmStoreAdminResponse::PublishedCurrentReleaseToStore { store_pid })
            }
            WasmStoreAdminCommand::PublishCurrentReleaseToCurrentStore => {
                Self::publish_current_release_set_to_current_store().await?;
                Ok(WasmStoreAdminResponse::PublishedCurrentReleaseToCurrentStore)
            }
            WasmStoreAdminCommand::SetPublicationBinding { binding } => {
                Self::set_current_publication_store_binding(binding.clone())?;
                Ok(WasmStoreAdminResponse::SetPublicationBinding { binding })
            }
            WasmStoreAdminCommand::ClearPublicationBinding => {
                Self::clear_current_publication_store_binding();
                Ok(WasmStoreAdminResponse::ClearedPublicationBinding)
            }
            WasmStoreAdminCommand::RetireDetachedBinding => {
                let binding = Self::retire_detached_publication_store_binding();
                Ok(WasmStoreAdminResponse::RetiredDetachedBinding { binding })
            }
            WasmStoreAdminCommand::PrepareRetiredStoreGc => {
                let binding = Self::prepare_retired_publication_store_for_gc().await?;
                Ok(WasmStoreAdminResponse::PreparedRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::BeginRetiredStoreGc => {
                let binding = Self::begin_retired_publication_store_gc().await?;
                Ok(WasmStoreAdminResponse::BeganRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::CompleteRetiredStoreGc => {
                let binding = Self::complete_retired_publication_store_gc().await?;
                Ok(WasmStoreAdminResponse::CompletedRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::FinalizeRetiredBinding => {
                let result = Self::finalize_retired_publication_store_binding()
                    .await?
                    .map(|(binding, store_pid)| WasmStoreFinalizedStoreResponse {
                        binding,
                        store_pid,
                    });
                Ok(WasmStoreAdminResponse::FinalizedRetiredBinding { result })
            }
            WasmStoreAdminCommand::DeleteFinalizedStore { binding, store_pid } => {
                Self::delete_finalized_publication_store(binding.clone(), store_pid).await?;
                Ok(WasmStoreAdminResponse::DeletedFinalizedStore { binding, store_pid })
            }
        }
    }
}
