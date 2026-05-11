// -----------------------------------------------------------------------------
// Topology endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for shared state snapshots.
#[macro_export]
macro_rules! canic_emit_topology_state_endpoints {
    () => {
        #[cfg(canic_is_root)]
        #[$crate::canic_query]
        fn canic_app_state() -> Result<::canic::dto::state::AppStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::AppStateQuery::snapshot())
        }

        #[cfg(canic_is_root)]
        #[$crate::canic_query]
        fn canic_subnet_state() -> Result<::canic::dto::state::SubnetStateResponse, ::canic::Error>
        {
            Ok($crate::__internal::core::api::state::SubnetStateQuery::snapshot())
        }
    };
}

// Leaf emitter for shared index views.
#[macro_export]
macro_rules! canic_emit_topology_index_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_app_index(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<
            ::canic::dto::page::Page<::canic::dto::topology::IndexEntryResponse>,
            ::canic::Error,
        > {
            Ok($crate::__internal::core::api::topology::index::AppIndexApi::page(page))
        }

        #[$crate::canic_query]
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

// Leaf emitter for the shared topology-children view.
#[macro_export]
macro_rules! canic_emit_topology_children_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_canister_children(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }
    };
}

// Leaf emitter for shared scaling/sharding placement views.
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

// Bundle composer for shared state, index, topology, and placement views.
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
