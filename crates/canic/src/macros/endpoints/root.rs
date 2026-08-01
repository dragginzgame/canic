//! Module: macros::endpoints::root
//!
//! Responsibility: emit root-canister endpoint macros for control and authority surfaces.
//! Does not own: root state, pool policy, auth proof issuance, or wasm-store workflows.
//! Boundary: exposes facade macros that delegate immediately to core/control-plane APIs.

/// Emit root-only control-plane, registry, and operator admin endpoints.
#[macro_export]
macro_rules! canic_emit_root_admin_endpoints {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_authority(
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootAuthority, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_authority()
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_canister_summary(
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_canister_summary()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_draining_begin(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootDrainingRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootDrainingResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_fleet_subnet_root_draining(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_draining_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootDrainingStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootDrainingResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_draining_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_draining_inventory_finalize(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::finalize_fleet_subnet_root_inventory(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_draining_inventory_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_final_inventory_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_removal_publish(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootRemovalRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::publish_fleet_subnet_root_removal(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_removal_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootRemovalStatusRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_removal_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_reclaim(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reclaim_fleet_subnet_root_store(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_reclamation_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreReclamationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_store_reclamation_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_binding_finalize(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::finalize_fleet_subnet_root_store_binding(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_binding_finalization_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreBindingFinalizationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_store_binding_finalization_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_delete(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::delete_fleet_subnet_root_store(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_store_deletion_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootStoreDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_store_deletion_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_deletion_prepare(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_fleet_subnet_root_deletion(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_subnet_root_deletion_preparation_status(
            request: ::canic::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationStatusRequest,
        ) -> Result<::canic::dto::fleet_subnet_root::FleetSubnetRootDeletionPreparationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_deletion_preparation_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_synchronize(
            request: ::canic::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::synchronize_fleet_registry(request).await
        }

        #[$crate::canic_query(composite, requires(caller::is_controller()))]
        async fn canic_fleet_registry_sync_status(
            request: ::canic::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_registry_sync_status(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_activate_mirror(
            request: ::canic::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_fleet_registry_mirror(request).await
        }

        #[$crate::canic_query(composite, requires(caller::is_controller()))]
        async fn canic_fleet_registry_mirror_status(
            request: ::canic::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_registry_mirror_status(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_registry_prepare(
            request: ::canic::dto::component_registry::RootComponentRegistryPreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentRegistryStatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_registry(request).await
        }

        #[$crate::canic_query(composite, requires(caller::is_controller()))]
        async fn canic_root_component_registry_status(
            request: ::canic::dto::component_registry::RootComponentRegistryPreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentRegistryStatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_status(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_allocate(
            request: ::canic::dto::component_registry::RootComponentAllocationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_allocation(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_allocation_status(
            request: ::canic::dto::component_registry::RootComponentAllocationStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_allocation_status(request)
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_allocate(
            request: ::canic::dto::component_registry::RootComponentChildAllocationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_child(request).await
        }

        #[$crate::canic_query(internal, public)]
        async fn canic_root_component_child_allocation_status(
            request: ::canic::dto::component_registry::RootComponentChildAllocationStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_child_allocation_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_draining_begin(
            request: ::canic::dto::component_registry::RootComponentDrainingRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDrainingResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_draining(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_draining_status(
            request: ::canic::dto::component_registry::RootComponentDrainingStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDrainingResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_draining_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_quiesce(
            request: ::canic::dto::component_registry::RootComponentQuiescenceRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentQuiescenceResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::quiesce_component(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_quiescence_status(
            request: ::canic::dto::component_registry::RootComponentQuiescenceStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentQuiescenceResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_quiescence_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_draining_advance(
            request: ::canic::dto::component_registry::RootComponentDrainingAdvanceRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDrainingAdvanceResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::advance_component_draining(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_draining_inventory_finalize(
            request: ::canic::dto::component_registry::RootComponentFinalInventoryRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentFinalInventoryResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::finalize_component_inventory(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_delete(
            request: ::canic::dto::component_registry::RootComponentDeletionRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::delete_component(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_membership_remove(
            request: ::canic::dto::component_registry::RootComponentDeletionRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::remove_component_membership(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_deletion_status(
            request: ::canic::dto::component_registry::RootComponentDeletionStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_deletion_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_begin(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_subtree_removal(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_advance(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalAdvanceRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::advance_component_subtree_removal(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_stop_prepare(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalStopPreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_subtree_leaf_stop(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_stop(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalStopRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::stop_component_subtree_leaf(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_delete_prepare(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalDeletePreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_subtree_leaf_delete(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_delete(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalDeleteRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::delete_component_subtree_leaf(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_membership_remove(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalMembershipRemovalRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::remove_component_subtree_leaf_membership(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_directory_synchronize(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalDirectorySynchronizationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::synchronize_component_subtree_leaf_directory(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_leaf_finalize(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalLeafFinalizationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::finalize_component_subtree_leaf(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_subtree_removal_status(
            request: ::canic::dto::component_registry::RootComponentSubtreeRemovalStatusRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentSubtreeRemovalResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_subtree_removal_status(request)
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_create(
            request: ::canic::dto::component_registry::RootComponentChildCreationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::create_component_child(request).await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_install(
            request: ::canic::dto::component_registry::RootComponentChildInstallRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::install_component_child(request).await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_commit(
            request: ::canic::dto::component_registry::RootComponentChildCommitRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildCommitResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::commit_component_child(request).await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_directory_prepare(
            request: ::canic::dto::component_registry::RootComponentChildDirectoryPreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildDirectoryPreparationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_child_directories(request).await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_runtime_activate(
            request: ::canic::dto::component_registry::RootComponentChildRuntimeActivationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildRuntimeActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_component_child_runtime(request).await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_root_component_child_membership_activate(
            request: ::canic::dto::component_registry::RootComponentChildMembershipActivationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentChildMembershipActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_component_child_membership(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_create(
            request: ::canic::dto::component_registry::RootComponentCreationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::create_component_allocation(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_install(
            request: ::canic::dto::component_registry::RootComponentInstallRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentAllocationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::install_component_allocation(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_commit(
            request: ::canic::dto::component_registry::RootComponentCommitRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentCommitResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::commit_component_allocation(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_directory_prepare(
            request: ::canic::dto::component_registry::RootComponentDirectoryPreparationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentDirectoryPreparationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_directories(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_runtime_activate(
            request: ::canic::dto::component_registry::RootComponentRuntimeActivationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentRuntimeActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_component_runtime(request).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_component_membership_activate(
            request: ::canic::dto::component_registry::RootComponentMembershipActivationRequest,
        ) -> Result<::canic::dto::component_registry::RootComponentMembershipActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_component_membership(request).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_registry_partition(
            request: ::canic::dto::component_registry::ComponentRegistryPartitionRequest,
        ) -> Result<::canic::dto::component_registry::ComponentRegistryPartitionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_partition(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_component_directory_head(
            request: ::canic::dto::component_registry::ComponentDirectoryHeadRequest,
        ) -> Result<::canic::dto::component_registry::ComponentDirectoryHead, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_head(request)
        }

        #[$crate::canic_query(internal, public)]
        async fn canic_root_component_directory_page(
            request: ::canic::dto::component_registry::ComponentDirectoryPageRequest,
        ) -> Result<::canic::dto::component_registry::ComponentDirectoryPageResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_page(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_prepare_fleet_activation(
        ) -> Result<::canic::dto::fleet_activation::FleetActivationStatusResponse, ::canic::Error> {
            __canic_run_prepared_root_init_block().await;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_fleet_activation()
                .await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_resume_fleet_activation(
            request: ::canic::dto::fleet_activation::FleetActivationResumeRequest,
        ) -> Result<::canic::dto::fleet_activation::FleetActivationStatusResponse, ::canic::Error> {
            let transition = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::resume_fleet_activation(
                request,
            )
            .await?;
            __canic_schedule_prepared_activation_init();
            Ok(transition.status)
        }

        #[$crate::canic_update(internal, requires(caller::is_controller()))]
        async fn canic_fleet_admin(
            cmd: ::canic::dto::state::FleetCommand,
        ) -> Result<::canic::dto::state::FleetCommandResponse, ::canic::Error> {
            $crate::__internal::core::api::state::FleetStateApi::execute_command(cmd).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_upgrade(
            canister_pid: ::canic::__internal::cdk::Principal,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::__internal::cdk::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_icp_refill(
            request: ::canic::dto::icp_refill::IcpRefillRequest,
        ) -> Result<::canic::dto::icp_refill::IcpRefillEndpointResponse, ::canic::Error> {
            $crate::__internal::core::api::icp_refill::IcpRefillApi::refill(request).await
        }

        #[$crate::canic_query(public)]
        fn canic_subnet_registry()
        -> Result<::canic::dto::topology::SubnetRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::SubnetRegistryApi::registry())
        }

        #[$crate::canic_query(public)]
        async fn canic_pool_list()
        -> Result<::canic::dto::pool::CanisterPoolResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::pool::CanisterPoolApi::list())
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_pool_admin(
            cmd: ::canic::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::dto::pool::PoolAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::pool::CanisterPoolApi::admin(cmd).await
        }
    };
}

/// Emit root-only auth, delegation, and attestation authority endpoints.
#[macro_export]
macro_rules! canic_emit_root_auth_attestation_endpoints {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_upsert_root_issuer_policy(
            request: ::canic::dto::auth::RootIssuerPolicyUpsertRequest,
        ) -> Result<::canic::dto::auth::RootIssuerPolicyResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_policy_root(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_upsert_root_issuer_renewal_template(
            request: ::canic::dto::auth::RootIssuerRenewalTemplateUpsertRequest,
        ) -> Result<::canic::dto::auth::RootIssuerRenewalTemplateResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_renewal_template_root(
                request,
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_issuer_renewal_status(
            request: ::canic::dto::auth::RootIssuerRenewalStatusRequest,
        ) -> Result<::canic::dto::auth::RootIssuerRenewalStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::root_issuer_renewal_status_root(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_get_or_create_chain_key_delegation_proof(
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchProof, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_or_create_chain_key_delegation_proof_root()
                .await
        }

        #[$crate::canic_update(internal, public)]
        async fn canic_prepare_role_attestation(
            request: ::canic::dto::auth::RoleAttestationRequest,
        ) -> Result<::canic::dto::auth::RoleAttestationPrepareResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::prepare_role_attestation(request)
        }

        #[$crate::canic_query(internal, public)]
        async fn canic_get_role_attestation(
            request: ::canic::dto::auth::RoleAttestationGetRequest,
        ) -> Result<::canic::dto::auth::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::get_role_attestation(request)
        }
    };
}

/// Emit root-only wasm-store bootstrap and publication control endpoints.
#[macro_export]
macro_rules! canic_emit_root_wasm_store_endpoints {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_debug(
        ) -> Result<::canic::dto::template::WasmStoreBootstrapDebugResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::debug_bootstrap()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_root_store_bootstrap(
            request: ::canic::dto::root_store::RootStoreBootstrapRequest,
        ) -> Result<::canic::dto::root_store::RootStoreBootstrapResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::bootstrap_root_store(request)
                .await
        }

        #[$crate::canic_query(composite, requires(caller::is_controller()))]
        async fn canic_root_store_bootstrap_status(
            request: ::canic::dto::root_store::RootStoreBootstrapRequest,
        ) -> Result<::canic::dto::root_store::RootStoreBootstrapResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::root_store_status(request)
                .await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_template_stage_manifest_admin(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::stage_manifest(request);
            Ok(())
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_template_prepare_admin(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_chunk_set(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()), payload(max_bytes = ::canic::CANIC_WASM_CHUNK_BYTES + 64 * 1024))]
        async fn canic_template_publish_chunk_admin(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_chunk(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_admin(
            cmd: ::canic::dto::template::WasmStoreAdminCommand,
        ) -> Result<::canic::dto::template::WasmStoreAdminResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::admin(cmd).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_overview(
        ) -> Result<::canic::dto::template::WasmStoreOverviewResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::overview()
        }

    };
}
