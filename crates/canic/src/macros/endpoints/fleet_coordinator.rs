//! Module: macros::endpoints::fleet_coordinator
//!
//! Responsibility: emit the dedicated Fleet Coordinator endpoint surface.
//! Does not own: Registry state, validation, lifecycle orchestration, or root behavior.
//! Boundary: every export delegates immediately to the Coordinator API facade.

/// Emit the controller-facing Fleet Coordinator endpoint surface.
#[macro_export]
macro_rules! canic_emit_fleet_coordinator_endpoints {
    () => {
        $crate::canic_emit_authority_restore_endpoints!();

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry(
        ) -> Result<::canic::dto::fleet_registry::FleetRegistry, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::registry()
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry_manifest(
        ) -> Result<::canic::dto::fleet_registry::FleetRegistryManifest, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::manifest()
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry_version(
        ) -> Result<::canic::dto::fleet_registry::FleetRegistryVersion, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::version()
        }

        #[$crate::canic_update(
            requires(caller::is_controller()),
            payload(max_bytes = ::canic::__internal::core::control_plane_support::ops::fleet_registry::MAX_FLEET_REGISTRY_CANONICAL_BYTES)
        )]
        async fn canic_fleet_subnet_root_join(
            request: ::canic::dto::fleet_registry::FleetSubnetRootJoinRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootJoinResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::join_root(
                request,
            )
        }

        #[$crate::canic_update(public)]
        async fn canic_fleet_registry_snapshot_for_root(
        ) -> Result<::canic::dto::fleet_registry::FleetRegistrySnapshotResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::snapshot_for_calling_root()
        }

        #[$crate::canic_update(public)]
        async fn canic_fleet_registry_acknowledge_root(
            request: ::canic::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgement, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::acknowledge_calling_root_snapshot(
                request,
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_acknowledgements(
        ) -> Result<Vec<::canic::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgement>, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_snapshot_acknowledgements()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_activate(
            request: ::canic::dto::fleet_registry::FleetRegistryActivationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetRegistryActivationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::activate_registry(
                request,
            )
        }

        #[$crate::canic_update(
            requires(caller::is_controller()),
            payload(max_bytes = ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES)
        )]
        async fn canic_fleet_component_provisioning_prepare(
            request: ::canic::dto::component_provisioning::FleetComponentProvisioningPrepareRequest,
        ) -> Result<::canic::dto::component_provisioning::FleetComponentProvisioningStatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::prepare_component_provisioning(
                request,
            )
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_component_provisioning_advance(
            request: ::canic::dto::component_provisioning::FleetComponentProvisioningAdvanceRequest,
        ) -> Result<::canic::dto::component_provisioning::FleetComponentProvisioningStatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::advance_component_provisioning(
                request,
            ).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_component_provisioning_status(
            request: ::canic::dto::component_provisioning::FleetComponentProvisioningStatusRequest,
        ) -> Result<::canic::dto::component_provisioning::FleetComponentProvisioningStatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::component_provisioning_status(
                request,
            )
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_publish_root_draining(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDrainingPublicationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::publish_root_draining(
                request,
            )
        }


        #[$crate::canic_update(public)]
        async fn canic_fleet_registry_publish_root_removed(
            request: ::canic::dto::fleet_registry::FleetSubnetRootRemovalPublicationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::publish_root_removed(
                request,
            )
        }

        #[$crate::canic_update(public)]
        async fn canic_fleet_registry_root_deletion_readiness_prepare(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::prepare_root_deletion_readiness(request)
        }

        #[$crate::canic_update(public)]
        async fn canic_fleet_registry_root_deletion_ready(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionReadinessRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::record_root_deletion_readiness(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_deletion_execution_begin(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionExecutionRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionExecutionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::begin_root_deletion_execution(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_deletion_execution_status(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionStatusRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionExecutionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_deletion_execution_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_deletion_complete(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionCompletionRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::complete_root_deletion(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_deletion_status(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDeletionStatusRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDeletionResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_deletion_status(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_fleet_registry_root_draining_reservation_prepare(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDrainingReservationRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDrainingReservationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::prepare_root_draining_reservation(request)
        }

        #[$crate::canic_query(public)]
        async fn canic_fleet_registry_root_draining_reservation_status(
            request: ::canic::dto::fleet_registry::FleetSubnetRootDrainingReservationStatusRequest,
        ) -> Result<::canic::dto::fleet_registry::FleetSubnetRootDrainingReservationResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_draining_reservation_status(request)
        }
    };
}
