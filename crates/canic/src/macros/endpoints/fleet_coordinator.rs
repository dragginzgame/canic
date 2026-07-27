//! Module: macros::endpoints::fleet_coordinator
//!
//! Responsibility: emit the dedicated Fleet Coordinator endpoint surface.
//! Does not own: Registry state, validation, lifecycle orchestration, or root behavior.
//! Boundary: every export delegates immediately to the Coordinator API facade.

/// Emit controller-facing canonical Fleet Registry query endpoints.
#[macro_export]
macro_rules! canic_emit_fleet_coordinator_endpoints {
    () => {
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
    };
}
