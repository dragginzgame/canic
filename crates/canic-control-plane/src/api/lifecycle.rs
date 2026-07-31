use canic_core::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap::{EmbeddedRootBootstrapEntry, compiled::ConfigModel},
    control_plane_support::view::fleet_activation::FleetActivationTransition,
    dto::fleet_registry::{
        FleetSubnetRootRegistryMirrorActivationRequest,
        FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
        FleetSubnetRootRegistrySyncResponse, FleetSubnetRootRemovalPublicationResponse,
    },
    dto::fleet_subnet_root::{
        FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootDrainingRequest,
        FleetSubnetRootDrainingResponse, FleetSubnetRootDrainingStatusRequest,
        FleetSubnetRootFinalInventoryRequest, FleetSubnetRootFinalInventoryResponse,
        FleetSubnetRootFinalInventoryStatusRequest, FleetSubnetRootInitArgs,
        FleetSubnetRootRemovalRequest, FleetSubnetRootRemovalStatusRequest,
    },
    dto::{
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentDirectoryPageRequest,
            ComponentDirectoryPageResponse, ComponentRegistryPartitionRequest,
            ComponentRegistryPartitionResponse, RootComponentAllocationRequest,
            RootComponentAllocationResponse, RootComponentAllocationStatusRequest,
            RootComponentChildAllocationRequest, RootComponentChildAllocationResponse,
            RootComponentChildAllocationStatusRequest, RootComponentChildCommitRequest,
            RootComponentChildCommitResponse, RootComponentChildCreationRequest,
            RootComponentChildDirectoryPreparationRequest,
            RootComponentChildDirectoryPreparationResponse, RootComponentChildInstallRequest,
            RootComponentChildMembershipActivationRequest,
            RootComponentChildMembershipActivationResponse,
            RootComponentChildRuntimeActivationRequest,
            RootComponentChildRuntimeActivationResponse, RootComponentCommitRequest,
            RootComponentCommitResponse, RootComponentCreationRequest,
            RootComponentDeletionRequest, RootComponentDeletionResponse,
            RootComponentDeletionStatusRequest, RootComponentDirectoryPreparationRequest,
            RootComponentDirectoryPreparationResponse, RootComponentDrainingAdvanceRequest,
            RootComponentDrainingAdvanceResponse, RootComponentDrainingRequest,
            RootComponentDrainingResponse, RootComponentDrainingStatusRequest,
            RootComponentFinalInventoryRequest, RootComponentFinalInventoryResponse,
            RootComponentInstallRequest, RootComponentMembershipActivationRequest,
            RootComponentMembershipActivationResponse, RootComponentQuiescenceRequest,
            RootComponentQuiescenceResponse, RootComponentQuiescenceStatusRequest,
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
            RootComponentRuntimeActivationRequest, RootComponentRuntimeActivationResponse,
            RootComponentSubtreeRemovalAdvanceRequest,
            RootComponentSubtreeRemovalDeletePreparationRequest,
            RootComponentSubtreeRemovalDeleteRequest,
            RootComponentSubtreeRemovalDirectorySynchronizationRequest,
            RootComponentSubtreeRemovalLeafFinalizationRequest,
            RootComponentSubtreeRemovalMembershipRemovalRequest,
            RootComponentSubtreeRemovalRequest, RootComponentSubtreeRemovalResponse,
            RootComponentSubtreeRemovalStatusRequest,
            RootComponentSubtreeRemovalStopPreparationRequest,
            RootComponentSubtreeRemovalStopRequest,
        },
        fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
    },
};
use std::time::Duration;

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    /// Delegate root init-time runtime seeding to the current core implementation.
    pub fn init_root_canister_before_bootstrap(
        args: FleetSubnetRootInitArgs,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        embedded_wasm_store_bootstrap_release_set: &'static [EmbeddedRootBootstrapEntry],
    ) {
        crate::api::template::WasmStoreBootstrapApi::register_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        crate::runtime::install::register_template_module_source_resolver();
        canic_core::api::lifecycle::root::LifecycleApi::init_root_canister_before_bootstrap(
            args,
            config,
            config_source,
            config_path,
        );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
    }

    pub fn fleet_subnet_root_authority()
    -> Result<FleetSubnetRootAuthority, canic_core::dto::error::Error> {
        canic_core::api::fleet_activation::FleetActivationApi::root_authority()
    }

    pub fn fleet_subnet_root_canister_summary()
    -> Result<FleetSubnetRootCanisterSummary, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::canister_summary().map_err(Into::into)
    }

    pub fn begin_fleet_subnet_root_draining(
        request: FleetSubnetRootDrainingRequest,
    ) -> Result<FleetSubnetRootDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::begin_draining(request).map_err(Into::into)
    }

    pub fn fleet_subnet_root_draining_status(
        request: FleetSubnetRootDrainingStatusRequest,
    ) -> Result<FleetSubnetRootDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::draining_status(request).map_err(Into::into)
    }

    pub async fn finalize_fleet_subnet_root_inventory(
        request: FleetSubnetRootFinalInventoryRequest,
    ) -> Result<FleetSubnetRootFinalInventoryResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::finalize_inventory(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_final_inventory_status(
        request: FleetSubnetRootFinalInventoryStatusRequest,
    ) -> Result<FleetSubnetRootFinalInventoryResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::final_inventory_status(request).map_err(Into::into)
    }

    pub async fn publish_fleet_subnet_root_removal(
        request: FleetSubnetRootRemovalRequest,
    ) -> Result<FleetSubnetRootRemovalPublicationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::publish_removal(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_removal_status(
        request: FleetSubnetRootRemovalStatusRequest,
    ) -> Result<FleetSubnetRootRemovalPublicationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::removal_status(request).map_err(Into::into)
    }

    pub async fn synchronize_fleet_registry(
        request: FleetSubnetRootRegistrySyncRequest,
    ) -> Result<FleetSubnetRootRegistrySyncResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_registry_mirror::synchronize(request)
            .await
            .map_err(Into::into)
    }

    pub async fn fleet_registry_sync_status(
        request: FleetSubnetRootRegistrySyncRequest,
    ) -> Result<FleetSubnetRootRegistrySyncResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_registry_mirror::status(request)
            .await
            .map_err(Into::into)
    }

    pub async fn activate_fleet_registry_mirror(
        request: FleetSubnetRootRegistryMirrorActivationRequest,
    ) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, canic_core::dto::error::Error>
    {
        crate::workflow::fleet_registry_mirror::activate(request)
            .await
            .map_err(Into::into)
    }

    pub async fn fleet_registry_mirror_status(
        request: FleetSubnetRootRegistryMirrorActivationRequest,
    ) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, canic_core::dto::error::Error>
    {
        crate::workflow::fleet_registry_mirror::active_status(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_registry(
        request: RootComponentRegistryPreparationRequest,
    ) -> Result<RootComponentRegistryStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::prepare(request)
            .await
            .map_err(Into::into)
    }

    pub async fn component_registry_status(
        request: RootComponentRegistryPreparationRequest,
    ) -> Result<RootComponentRegistryStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::status(request)
            .await
            .map_err(Into::into)
    }

    pub async fn reserve_component_allocation(
        request: RootComponentAllocationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_allocation_status(
        request: RootComponentAllocationStatusRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::allocation_status(request).map_err(Into::into)
    }

    pub async fn reserve_component_child(
        request: RootComponentChildAllocationRequest,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_child_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_child_allocation_status(
        request: RootComponentChildAllocationStatusRequest,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::child_allocation_status(request).map_err(Into::into)
    }

    pub async fn begin_component_draining(
        request: RootComponentDrainingRequest,
    ) -> Result<RootComponentDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::begin_component_draining(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_draining_status(
        request: RootComponentDrainingStatusRequest,
    ) -> Result<RootComponentDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_draining_status(request).map_err(Into::into)
    }

    pub async fn quiesce_component(
        request: RootComponentQuiescenceRequest,
    ) -> Result<RootComponentQuiescenceResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::quiesce_component(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_quiescence_status(
        request: RootComponentQuiescenceStatusRequest,
    ) -> Result<RootComponentQuiescenceResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_quiescence_status(request)
            .map_err(Into::into)
    }

    pub async fn advance_component_draining(
        request: RootComponentDrainingAdvanceRequest,
    ) -> Result<RootComponentDrainingAdvanceResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::advance_component_draining(request)
            .await
            .map_err(Into::into)
    }

    pub async fn finalize_component_inventory(
        request: RootComponentFinalInventoryRequest,
    ) -> Result<RootComponentFinalInventoryResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::finalize_component_inventory(request)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_component(
        request: RootComponentDeletionRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::delete_component(request)
            .await
            .map_err(Into::into)
    }

    pub fn remove_component_membership(
        request: RootComponentDeletionRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::remove_component_membership(request)
            .map_err(Into::into)
    }

    pub fn component_deletion_status(
        request: RootComponentDeletionStatusRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_deletion_status(request).map_err(Into::into)
    }

    pub async fn begin_component_subtree_removal(
        request: RootComponentSubtreeRemovalRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::begin_subtree_removal(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_subtree_removal_status(
        request: RootComponentSubtreeRemovalStatusRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::subtree_removal_status(request).map_err(Into::into)
    }

    pub async fn advance_component_subtree_removal(
        request: RootComponentSubtreeRemovalAdvanceRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::advance_subtree_removal(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_subtree_leaf_stop(
        request: RootComponentSubtreeRemovalStopPreparationRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::prepare_subtree_leaf_stop(request)
            .await
            .map_err(Into::into)
    }

    pub async fn stop_component_subtree_leaf(
        request: RootComponentSubtreeRemovalStopRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::stop_subtree_leaf(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_subtree_leaf_delete(
        request: RootComponentSubtreeRemovalDeletePreparationRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::prepare_subtree_leaf_delete(request)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_component_subtree_leaf(
        request: RootComponentSubtreeRemovalDeleteRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::delete_subtree_leaf(request)
            .await
            .map_err(Into::into)
    }

    pub async fn remove_component_subtree_leaf_membership(
        request: RootComponentSubtreeRemovalMembershipRemovalRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::remove_subtree_leaf_membership(request)
            .await
            .map_err(Into::into)
    }

    pub async fn synchronize_component_subtree_leaf_directory(
        request: RootComponentSubtreeRemovalDirectorySynchronizationRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::synchronize_subtree_leaf_directory(request)
            .await
            .map_err(Into::into)
    }

    pub async fn finalize_component_subtree_leaf(
        request: RootComponentSubtreeRemovalLeafFinalizationRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::finalize_subtree_leaf(request)
            .await
            .map_err(Into::into)
    }

    pub async fn create_component_child(
        request: RootComponentChildCreationRequest,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::create_child_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn install_component_child(
        request: RootComponentChildInstallRequest,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::install_child_allocation(request))
            .await
            .map_err(Into::into)
    }

    pub async fn commit_component_child(
        request: RootComponentChildCommitRequest,
    ) -> Result<RootComponentChildCommitResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::commit_child_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_child_directories(
        request: RootComponentChildDirectoryPreparationRequest,
    ) -> Result<RootComponentChildDirectoryPreparationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::prepare_child_directories(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_child_runtime(
        request: RootComponentChildRuntimeActivationRequest,
    ) -> Result<RootComponentChildRuntimeActivationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::activate_child_runtime(request)
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_child_membership(
        request: RootComponentChildMembershipActivationRequest,
    ) -> Result<RootComponentChildMembershipActivationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::activate_child_membership(request)
            .await
            .map_err(Into::into)
    }

    pub async fn create_component_allocation(
        request: RootComponentCreationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::create_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn install_component_allocation(
        request: RootComponentInstallRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::install_allocation(
            request,
        ))
        .await
        .map_err(Into::into)
    }

    pub async fn commit_component_allocation(
        request: RootComponentCommitRequest,
    ) -> Result<RootComponentCommitResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::commit_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_directories(
        request: RootComponentDirectoryPreparationRequest,
    ) -> Result<RootComponentDirectoryPreparationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::prepare_component_directories(request)
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_runtime(
        request: RootComponentRuntimeActivationRequest,
    ) -> Result<RootComponentRuntimeActivationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::activate_component_runtime(request)
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_membership(
        request: RootComponentMembershipActivationRequest,
    ) -> Result<RootComponentMembershipActivationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::activate_component_membership(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_registry_partition(
        request: ComponentRegistryPartitionRequest,
    ) -> Result<ComponentRegistryPartitionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::registry_partition(request).map_err(Into::into)
    }

    pub fn component_directory_head(
        request: ComponentDirectoryHeadRequest,
    ) -> Result<ComponentDirectoryHead, canic_core::dto::error::Error> {
        crate::workflow::component_registry::directory_head(request).map_err(Into::into)
    }

    pub fn component_directory_page(
        request: ComponentDirectoryPageRequest,
    ) -> Result<ComponentDirectoryPageResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::directory_page(request).map_err(Into::into)
    }

    pub async fn prepare_fleet_activation()
    -> Result<FleetActivationStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::bootstrap::root::bootstrap_init_root_canister().await;
        if !crate::workflow::bootstrap::root::activation_preparation_complete() {
            return Err(canic_core::dto::error::Error::unavailable(
                "root bootstrap has not prepared the complete managed inventory; inspect bootstrap status and retry activation preparation",
            ));
        }
        let current = canic_core::api::fleet_activation::FleetActivationApi::status()?;
        if current.phase == FleetActivationPhase::Active {
            crate::workflow::component_registry::mark_root_runtime_activated(
                current.identity.operation_id,
            )
            .map_err(canic_core::dto::error::Error::from)?;
            return Ok(current);
        }
        crate::workflow::component_registry::seal_root_activation_inventory(
            current.identity.operation_id,
        )
        .await
        .map_err(canic_core::dto::error::Error::from)?;
        canic_core::api::fleet_activation::FleetActivationApi::prepare_root().await
    }

    pub async fn resume_fleet_activation(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, canic_core::dto::error::Error> {
        let current = canic_core::api::fleet_activation::FleetActivationApi::status()?;
        if current.phase == FleetActivationPhase::Prepared {
            crate::workflow::component_registry::converge_root_activation_inventory(
                request.operation_id,
            )
            .await
            .map_err(canic_core::dto::error::Error::from)?;
        }
        let transition =
            canic_core::api::fleet_activation::FleetActivationApi::resume_root(request).await?;
        if transition.status.phase != FleetActivationPhase::Active {
            return Err(canic_core::dto::error::Error::unavailable(
                "Fleet activation resume did not activate the root runtime",
            ));
        }
        crate::workflow::component_registry::mark_root_runtime_activated(request.operation_id)
            .map_err(canic_core::dto::error::Error::from)?;
        crate::workflow::bootstrap::root::bootstrap_init_root_canister().await;
        if !canic_core::api::ready::ReadyApi::is_ready() {
            return Err(canic_core::dto::error::Error::unavailable(
                "Fleet activation completed but root bootstrap is not ready; inspect bootstrap status and retry activation resume",
            ));
        }
        Ok(transition)
    }

    /// Delegate root post-upgrade runtime restore to the current core implementation.
    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        embedded_wasm_store_bootstrap_release_set: &'static [EmbeddedRootBootstrapEntry],
    ) -> bool {
        crate::api::template::WasmStoreBootstrapApi::register_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        crate::runtime::install::register_template_module_source_resolver();
        let active =
            canic_core::api::lifecycle::root::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                config,
                config_source,
                config_path,
            );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        active
    }

    /// Delegate root post-upgrade bootstrap scheduling to the current core implementation.
    pub fn schedule_post_upgrade_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::defer_lifecycle(
            Duration::ZERO,
            "canic:bootstrap:post_upgrade_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
            },
        );
    }
}
