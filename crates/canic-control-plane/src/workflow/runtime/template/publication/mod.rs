mod fleet;
mod lifecycle;
mod release;
mod store;

use crate::{
    dto::template::{
        WasmStoreAdminCommand, WasmStoreAdminResponse, WasmStoreFinalizedStoreResponse,
        WasmStorePublicationSlotResponse, WasmStorePublicationStatusResponse,
        WasmStorePublicationStoreStatusResponse,
    },
    ids::CanisterRole,
    ops::storage::state::subnet::SubnetStateOps,
};
use canic_core::__control_plane_core as cp_core;
use cp_core::InternalError;
use std::collections::BTreeMap;

const WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

///
/// WasmStorePublicationWorkflow
///

pub struct WasmStorePublicationWorkflow;

impl WasmStorePublicationWorkflow {
    const WASM_STORE_CAPACITY_EXCEEDED_MESSAGE: &str = "wasm store capacity exceeded";

    // Return one root-facing live publication snapshot that explains slot state and candidate order.
    pub async fn publication_status() -> Result<WasmStorePublicationStatusResponse, InternalError> {
        let managed_manifests = Self::managed_release_manifests()?;
        let fleet = Self::snapshot_publication_store_fleet().await?;
        let publication = SubnetStateOps::publication_store_state_response();
        let writable_indices = fleet.writable_store_indices();
        let mut candidate_orders = BTreeMap::new();

        for (order, index) in writable_indices.into_iter().enumerate() {
            let order = u32::try_from(order).unwrap_or(u32::MAX);
            candidate_orders.insert(index, order);
        }

        let stores = fleet
            .stores
            .iter()
            .enumerate()
            .map(|(index, store)| {
                let exact_managed_release_count = u32::try_from(
                    managed_manifests
                        .iter()
                        .filter(|manifest| store.has_exact_release(manifest))
                        .count(),
                )
                .unwrap_or(u32::MAX);
                let conflicting_managed_release_count = u32::try_from(
                    managed_manifests
                        .iter()
                        .filter(|manifest| store.conflicting_release(manifest).is_some())
                        .count(),
                )
                .unwrap_or(u32::MAX);
                let publication_slot =
                    if publication.active_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Active)
                    } else if publication.detached_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Detached)
                    } else if publication.retired_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Retired)
                    } else {
                        None
                    };
                let is_reserved_for_publication = Self::binding_is_reserved_for_publication(
                    &fleet.reserved_state,
                    &store.binding,
                );

                WasmStorePublicationStoreStatusResponse {
                    binding: store.binding.clone(),
                    pid: store.pid,
                    created_at: store.created_at,
                    publication_slot,
                    is_preferred_binding: fleet.preferred_binding.as_ref() == Some(&store.binding),
                    is_reserved_for_publication,
                    is_selectable_for_publication: !is_reserved_for_publication,
                    publication_candidate_order: candidate_orders.get(&index).copied(),
                    exact_managed_release_count,
                    conflicting_managed_release_count,
                    store: store.status.clone(),
                }
            })
            .collect();

        Ok(WasmStorePublicationStatusResponse {
            publication,
            preferred_binding: fleet.preferred_binding,
            managed_release_count: u32::try_from(managed_manifests.len()).unwrap_or(u32::MAX),
            stores,
        })
    }

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
