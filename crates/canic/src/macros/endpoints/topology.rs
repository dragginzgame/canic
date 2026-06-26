//! Module: macros::endpoints::topology
//!
//! Responsibility: emit topology state, index, children, and placement views.
//! Does not own: topology state, placement policy, or authorization policy.
//! Boundary: exposes facade macros that delegate immediately to core APIs.

/// Emit root-only topology state snapshot endpoints.
#[macro_export]
macro_rules! canic_emit_topology_state_endpoints {
    () => {
        #[cfg(canic_is_root)]
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_app_state() -> Result<::canic::dto::state::AppStateResponse, ::canic::Error>
        {
            Ok($crate::__internal::core::api::state::AppStateQuery::snapshot())
        }

        #[cfg(canic_is_root)]
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_subnet_state()
        -> Result<::canic::dto::state::SubnetStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::SubnetStateQuery::snapshot())
        }
    };
}

/// Emit shared topology index query endpoints.
#[macro_export]
macro_rules! canic_emit_topology_index_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_app_index(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<
            ::canic::dto::page::Page<::canic::dto::topology::IndexEntryResponse>,
            ::canic::Error,
        > {
            Ok($crate::__internal::core::api::topology::index::AppIndexApi::page(page))
        }

        #[$crate::canic_query(public)]
        fn canic_subnet_index(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<
            ::canic::dto::page::Page<::canic::dto::topology::IndexEntryResponse>,
            ::canic::Error,
        > {
            Ok($crate::__internal::core::api::topology::index::SubnetIndexApi::page(page))
        }
    };
}

/// Emit the shared topology-children query endpoint.
#[macro_export]
macro_rules! canic_emit_topology_children_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_canister_children(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }
    };
}

/// Emit shared scaling and sharding placement view endpoints.
#[macro_export]
macro_rules! canic_emit_topology_placement_endpoints {
    () => {
        #[cfg(canic_has_scaling)]
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_scaling_registry()
        -> Result<::canic::dto::placement::scaling::ScalingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::scaling::ScalingApi::registry())
        }

        #[cfg(canic_has_sharding)]
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_registry()
        -> Result<::canic::dto::placement::sharding::ShardingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::registry())
        }

        #[cfg(canic_has_sharding)]
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_partition_keys(
            pool: String,
            shard_pid: ::canic::__internal::core::cdk::types::Principal,
        ) -> Result<::canic::dto::placement::sharding::ShardingPartitionKeysResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::partition_keys(&pool, shard_pid))
        }
    };
}

/// Emit the standard topology view endpoint bundle.
#[macro_export]
macro_rules! canic_bundle_topology_views_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_topology_state))]
        $crate::canic_emit_topology_state_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_index))]
        $crate::canic_emit_topology_index_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_children))]
        $crate::canic_emit_topology_children_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_placement))]
        $crate::canic_emit_topology_placement_endpoints!();
    };
}
