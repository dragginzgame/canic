use canic_core::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap::compiled::ConfigModel,
    control_plane_support::view::fleet_activation::FleetActivationTransition,
    dto::fleet_registry::{
        FleetSubnetRootRegistryMirrorActivationRequest,
        FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
        FleetSubnetRootRegistrySyncResponse, FleetSubnetRootRemovalPublicationResponse,
    },
    dto::fleet_subnet_root::{
        FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary,
        FleetSubnetRootDeletionPreparationRequest, FleetSubnetRootDeletionPreparationResponse,
        FleetSubnetRootDeletionPreparationStatusRequest, FleetSubnetRootDrainingRequest,
        FleetSubnetRootDrainingResponse, FleetSubnetRootDrainingStatusRequest,
        FleetSubnetRootFinalInventoryRequest, FleetSubnetRootFinalInventoryResponse,
        FleetSubnetRootFinalInventoryStatusRequest, FleetSubnetRootInitArgs,
        FleetSubnetRootRemovalRequest, FleetSubnetRootRemovalStatusRequest,
        FleetSubnetRootStoreBindingFinalizationRequest,
        FleetSubnetRootStoreBindingFinalizationResponse,
        FleetSubnetRootStoreBindingFinalizationStatusRequest, FleetSubnetRootStoreDeletionRequest,
        FleetSubnetRootStoreDeletionResponse, FleetSubnetRootStoreDeletionStatusRequest,
        FleetSubnetRootStoreReclamationRequest, FleetSubnetRootStoreReclamationResponse,
        FleetSubnetRootStoreReclamationStatusRequest, FleetSubnetWasmStoreAdoptionRequest,
        FleetSubnetWasmStoreAdoptionResponse,
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
            RootComponentSubtreeRemovalStopRequest, RootPeerComponentAllocationRequest,
        },
        fleet_activation::{FleetActivationResumeRequest, FleetActivationStatusResponse},
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
        embedded_release_build_id: Option<&str>,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        canic_core::api::timer::TimerApi::register_snapshot_resume_participant(
            crate::workflow::canister_pool::resume_after_authority_snapshot,
        );
        let canister_pool_config = args.authority.binding.limits.canister_pool.clone();
        let canister_pool_imports = args.canister_pool_imports.clone();
        let wasm_store = args.authority.wasm_store_authority.wasm_store;
        crate::runtime::install::register_template_module_source_resolver();
        canic_core::api::lifecycle::root::LifecycleApi::init_root_canister_before_bootstrap(
            args,
            embedded_release_build_id,
            config,
            config_source,
            config_path,
        );
        crate::workflow::canister_pool::declare();
        let now_ns = canic_core::control_plane_support::ops::ic::IcOps::now_nanos();
        crate::ops::canister_pool::CanisterPoolOps::initialize_store(wasm_store, now_ns)
            .and_then(|()| {
                crate::ops::canister_pool::CanisterPoolOps::initialize_imports(
                    &canister_pool_config,
                    &canister_pool_imports,
                    now_ns,
                )
            })
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("Canister pool initialization failed: {error}"))
            });
    }

    pub fn fleet_subnet_root_authority()
    -> Result<FleetSubnetRootAuthority, canic_core::dto::error::Error> {
        canic_core::api::fleet_activation::FleetActivationApi::root_authority()
    }

    pub fn fleet_subnet_root_canister_summary()
    -> Result<FleetSubnetRootCanisterSummary, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::canister_summary().map_err(Into::into)
    }

    pub async fn adopt_fleet_subnet_wasm_store(
        request: FleetSubnetWasmStoreAdoptionRequest,
    ) -> Result<FleetSubnetWasmStoreAdoptionResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::adopt_wasm_store(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_wasm_store_adoption_status(
        request: FleetSubnetWasmStoreAdoptionRequest,
    ) -> Result<FleetSubnetWasmStoreAdoptionResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::wasm_store_adoption_status(request).map_err(Into::into)
    }

    pub async fn begin_fleet_subnet_root_draining(
        request: FleetSubnetRootDrainingRequest,
    ) -> Result<FleetSubnetRootDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::begin_draining(request)
            .await
            .map_err(Into::into)
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

    pub async fn reclaim_fleet_subnet_root_store(
        request: FleetSubnetRootStoreReclamationRequest,
    ) -> Result<FleetSubnetRootStoreReclamationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::reclaim_store(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_store_reclamation_status(
        request: FleetSubnetRootStoreReclamationStatusRequest,
    ) -> Result<FleetSubnetRootStoreReclamationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::store_reclamation_status(request).map_err(Into::into)
    }

    pub async fn finalize_fleet_subnet_root_store_binding(
        request: FleetSubnetRootStoreBindingFinalizationRequest,
    ) -> Result<FleetSubnetRootStoreBindingFinalizationResponse, canic_core::dto::error::Error>
    {
        crate::workflow::fleet_subnet_root::finalize_store_binding(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_store_binding_finalization_status(
        request: FleetSubnetRootStoreBindingFinalizationStatusRequest,
    ) -> Result<FleetSubnetRootStoreBindingFinalizationResponse, canic_core::dto::error::Error>
    {
        crate::workflow::fleet_subnet_root::store_binding_finalization_status(request)
            .map_err(Into::into)
    }

    pub async fn delete_fleet_subnet_root_store(
        request: FleetSubnetRootStoreDeletionRequest,
    ) -> Result<FleetSubnetRootStoreDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::delete_store(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_store_deletion_status(
        request: FleetSubnetRootStoreDeletionStatusRequest,
    ) -> Result<FleetSubnetRootStoreDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::store_deletion_status(request).map_err(Into::into)
    }

    pub async fn prepare_fleet_subnet_root_deletion(
        request: FleetSubnetRootDeletionPreparationRequest,
    ) -> Result<FleetSubnetRootDeletionPreparationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::prepare_deletion(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_deletion_preparation_status(
        request: FleetSubnetRootDeletionPreparationStatusRequest,
    ) -> Result<FleetSubnetRootDeletionPreparationResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::deletion_preparation_status(request).map_err(Into::into)
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

    pub async fn reserve_peer_component_allocation(
        request: RootPeerComponentAllocationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_peer_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub fn peer_component_allocation_status(
        request: RootComponentAllocationStatusRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::peer_allocation_status(request).map_err(Into::into)
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
        Box::pin(crate::workflow::component_registry::quiesce_component(
            request,
        ))
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
        Box::pin(crate::workflow::component_registry::advance_component_draining(request))
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
        Box::pin(crate::workflow::component_registry::synchronize_subtree_leaf_directory(request))
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
        Box::pin(crate::workflow::component_registry::activate_child_membership(request))
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

    pub async fn create_peer_component_allocation(
        request: RootComponentCreationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::create_peer_allocation(request)
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

    pub async fn install_peer_component_allocation(
        request: RootComponentInstallRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::install_peer_allocation(request))
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

    pub async fn commit_peer_component_allocation(
        request: RootComponentCommitRequest,
    ) -> Result<RootComponentCommitResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::commit_peer_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_directories(
        request: RootComponentDirectoryPreparationRequest,
    ) -> Result<RootComponentDirectoryPreparationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::prepare_component_directories(request))
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_peer_component_directories(
        request: RootComponentDirectoryPreparationRequest,
    ) -> Result<RootComponentDirectoryPreparationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::prepare_peer_component_directories(request))
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

    pub async fn activate_peer_component_runtime(
        request: RootComponentRuntimeActivationRequest,
    ) -> Result<RootComponentRuntimeActivationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::activate_peer_component_runtime(request)
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_membership(
        request: RootComponentMembershipActivationRequest,
    ) -> Result<RootComponentMembershipActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_component_membership(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_peer_component_membership(
        request: RootComponentMembershipActivationRequest,
    ) -> Result<RootComponentMembershipActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_peer_component_membership(request))
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
        crate::workflow::runtime::fleet_activation::prepare_root()
            .await
            .map_err(Into::into)
    }

    pub async fn resume_fleet_activation(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, canic_core::dto::error::Error> {
        crate::workflow::runtime::fleet_activation::resume_root(request)
            .await
            .map_err(Into::into)
    }

    /// Delegate root post-upgrade runtime restore to the current core implementation.
    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        canic_core::api::timer::TimerApi::register_snapshot_resume_participant(
            crate::workflow::canister_pool::resume_after_authority_snapshot,
        );
        crate::runtime::install::register_template_module_source_resolver();
        let active =
            canic_core::api::lifecycle::root::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                config,
                config_source,
                config_path,
            );
        crate::workflow::canister_pool::declare();
        if active {
            crate::workflow::canister_pool::start().unwrap_or_else(|error| {
                ic_cdk::trap(format!("Canister pool maintenance start failed: {error}"))
            });
        }
        active
    }

    /// Delegate root post-upgrade bootstrap scheduling to the current core implementation.
    pub fn schedule_post_upgrade_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::defer_lifecycle_required(
            Duration::ZERO,
            "canic:bootstrap:post_upgrade_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
            },
        )
        .detach();
    }
}
