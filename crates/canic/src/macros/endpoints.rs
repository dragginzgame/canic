// -----------------------------------------------------------------------------
// Endpoint bundle macros
// -----------------------------------------------------------------------------

// Macros that generate public IC endpoints for Canic canisters.
// These bundles define the compile-time capability surface for `start!` and
// `start_root!`. The default compositions intentionally preserve the current
// feature set; bundle boundaries exist to make linker policy explicit.

// Lifecycle/runtime core shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_lifecycle_core {
    () => {
        #[canic_update(internal)]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        #[canic_query]
        fn canic_canister_cycle_balance() -> Result<u128, ::canic::Error> {
            Ok($crate::cdk::api::canister_cycle_balance())
        }

        #[canic_query]
        fn canic_canister_version() -> Result<u64, ::canic::Error> {
            Ok($crate::cdk::api::canister_version())
        }

        #[canic_query]
        fn canic_time() -> Result<u64, ::canic::Error> {
            Ok($crate::cdk::api::time())
        }

        #[canic_query(internal)]
        fn canic_ready() -> bool {
            $crate::__internal::core::api::ready::ReadyApi::is_ready()
        }
    };
}

// ICRC standards-facing query/update surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_standards_icrc {
    () => {
        #[canic_query(internal)]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[cfg(canic_icrc21_enabled)]
        #[canic_query(internal)]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }
    };
}

// ICTS metadata/status surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_standards_icts {
    () => {
        #[canic_query(internal)]
        fn icts_name() -> String {
            $crate::__internal::core::api::icts::IctsApi::name()
        }

        #[canic_query(internal)]
        fn icts_version() -> String {
            $crate::__internal::core::api::icts::IctsApi::version()
        }

        #[canic_query(internal)]
        fn icts_description() -> String {
            $crate::__internal::core::api::icts::IctsApi::description()
        }

        #[canic_query(internal)]
        fn icts_metadata() -> ::canic::dto::icts::CanisterMetadataResponse {
            $crate::__internal::core::api::icts::IctsApi::metadata()
        }
    };
}

// Combined standards-facing surface preserved for the default Canic runtime.
#[macro_export]
macro_rules! canic_endpoints_standards {
    () => {
        #[cfg(not(canic_disable_bundle_standards_icrc))]
        $crate::canic_endpoints_standards_icrc!();
        #[cfg(not(canic_disable_bundle_standards_icts))]
        $crate::canic_endpoints_standards_icts!();
    };
}

// Runtime memory-registry diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_observability_memory {
    () => {
        #[canic_query]
        fn canic_memory_registry()
        -> Result<::canic::dto::memory::MemoryRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::memory::MemoryQuery::registry())
        }
    };
}

// Environment snapshot diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_observability_env {
    () => {
        #[canic_query]
        fn canic_env() -> Result<::canic::dto::env::EnvSnapshotResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::env::EnvQuery::snapshot())
        }
    };
}

// Runtime log diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_observability_log {
    () => {
        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::__internal::core::log::Level>,
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::log::LogEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::log::LogQuery::page(
                crate_name, topic, min_level, page,
            ))
        }
    };
}

// Combined observability and operator-facing diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_observability {
    () => {
        #[cfg(not(canic_disable_bundle_observability_memory))]
        $crate::canic_endpoints_observability_memory!();
        #[cfg(not(canic_disable_bundle_observability_env))]
        $crate::canic_endpoints_observability_env!();
        #[cfg(not(canic_disable_bundle_observability_log))]
        $crate::canic_endpoints_observability_log!();
    };
}

// Metrics query surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_metrics {
    () => {
        #[canic_query]
        fn canic_metrics(
            kind: ::canic::dto::metrics::MetricsKind,
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::page(
                kind, page,
            ))
        }
    };
}

// Response capability and trust-chain runtime shared by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints_auth_attestation {
    () => {
        #[cfg(canic_is_root)]
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::RootCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::RootCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_root(envelope).await
        }

        #[cfg(not(canic_is_root))]
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::RootCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::RootCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(envelope)
                .await
        }
    };
}

// Shared state snapshots.
#[macro_export]
macro_rules! canic_endpoints_topology_state {
    () => {
        #[canic_query]
        fn canic_app_state() -> Result<::canic::dto::state::AppStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::AppStateQuery::snapshot())
        }

        #[canic_query]
        fn canic_subnet_state() -> Result<::canic::dto::state::SubnetStateResponse, ::canic::Error>
        {
            Ok($crate::__internal::core::api::state::SubnetStateQuery::snapshot())
        }
    };
}

// Shared directory views.
#[macro_export]
macro_rules! canic_endpoints_topology_directory {
    () => {
        #[canic_query]
        fn canic_app_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<
            ::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryResponse>,
            ::canic::Error,
        > {
            Ok($crate::__internal::core::api::topology::directory::AppDirectoryApi::page(page))
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<
            ::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryResponse>,
            ::canic::Error,
        > {
            Ok($crate::__internal::core::api::topology::directory::SubnetDirectoryApi::page(page))
        }
    };
}

// Shared topology children view.
#[macro_export]
macro_rules! canic_endpoints_topology_children {
    () => {
        #[canic_query]
        fn canic_canister_children(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }
    };
}

// Shared cycle-tracker view.
#[macro_export]
macro_rules! canic_endpoints_topology_cycles {
    () => {
        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }
    };
}

// Shared scaling/sharding placement views.
#[macro_export]
macro_rules! canic_endpoints_topology_placement {
    () => {
        #[cfg(canic_has_scaling)]
        #[canic_query(requires(caller::is_controller()))]
        async fn canic_scaling_registry()
        -> Result<::canic::dto::placement::scaling::ScalingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::scaling::ScalingApi::registry())
        }

        #[cfg(canic_has_sharding)]
        #[canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_registry()
        -> Result<::canic::dto::placement::sharding::ShardingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::registry())
        }

        #[cfg(canic_has_sharding)]
        #[canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_partition_keys(
            pool: String,
            shard_pid: ::canic::__internal::core::cdk::types::Principal,
        ) -> Result<::canic::dto::placement::sharding::ShardingPartitionKeysResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::partition_keys(&pool, shard_pid))
        }
    };
}

// Combined state, directory, topology, and placement views.
#[macro_export]
macro_rules! canic_endpoints_topology_views {
    () => {
        #[cfg(not(canic_disable_bundle_topology_state))]
        $crate::canic_endpoints_topology_state!();
        #[cfg(not(canic_disable_bundle_topology_directory))]
        $crate::canic_endpoints_topology_directory!();
        #[cfg(not(canic_disable_bundle_topology_children))]
        $crate::canic_endpoints_topology_children!();
        #[cfg(not(canic_disable_bundle_topology_cycles))]
        $crate::canic_endpoints_topology_cycles!();
        #[cfg(not(canic_disable_bundle_topology_placement))]
        $crate::canic_endpoints_topology_placement!();
    };
}

// Root-only control-plane, registry, and operator admin surface.
#[macro_export]
macro_rules! canic_endpoints_root_admin {
    () => {
        #[canic_update(internal, requires(caller::is_controller()))]
        async fn canic_app(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::state::AppStateApi::execute_command(cmd).await
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::dto::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res =
                $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid)
                    .await?;

            Ok(res)
        }

        #[canic_update(requires(caller::is_root(), caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        #[canic_query]
        fn canic_app_registry()
        -> Result<::canic::dto::topology::AppRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::AppRegistryApi::registry())
        }

        #[canic_query]
        fn canic_subnet_registry()
        -> Result<::canic::dto::topology::SubnetRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::SubnetRegistryApi::registry())
        }

        #[canic_query]
        async fn canic_pool_list()
        -> Result<::canic::dto::pool::CanisterPoolResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::pool::CanisterPoolApi::list())
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_pool_admin(
            cmd: ::canic::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::dto::pool::PoolAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::pool::CanisterPoolApi::admin(cmd).await
        }
    };
}

// Root-only auth, delegation, and attestation authority surface.
#[macro_export]
macro_rules! canic_endpoints_root_auth_attestation {
    () => {
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_delegation(
            request: ::canic::dto::auth::DelegationRequest,
        ) -> Result<::canic::dto::auth::DelegationProvisionResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::request_delegation(request).await
        }

        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_role_attestation(
            request: ::canic::dto::auth::RoleAttestationRequest,
        ) -> Result<::canic::dto::auth::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::request_role_attestation(request)
                .await
        }

        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_attestation_key_set()
        -> Result<::canic::dto::auth::AttestationKeySet, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::attestation_key_set().await
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_delegation_admin(
            cmd: ::canic::dto::auth::DelegationAdminCommand,
        ) -> Result<::canic::dto::auth::DelegationAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::admin(cmd).await
        }
    };
}

// Non-root sync surface for state and topology propagation.
#[macro_export]
macro_rules! canic_endpoints_nonroot_sync_topology {
    () => {
        #[canic_update(internal, requires(caller::is_parent()))]
        async fn canic_sync_state(
            snapshot: ::canic::dto::cascade::StateSnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot).await
        }

        #[canic_update(internal, requires(caller::is_parent()))]
        async fn canic_sync_topology(
            snapshot: ::canic::dto::cascade::TopologySnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_topology(snapshot).await
        }
    };
}

// Non-root auth/attestation provisioning surface.
#[macro_export]
macro_rules! canic_endpoints_nonroot_auth_attestation {
    () => {
        #[cfg(canic_accepts_delegation_signer_proof)]
        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_set_signer_proof(
            request: ::canic::dto::auth::DelegationProofInstallRequest,
        ) -> Result<(), ::canic::Error> {
            let self_pid = $crate::__internal::core::cdk::api::canister_self();
            if request.proof.cert.shard_pid != self_pid {
                return Err(::canic::Error::invalid(
                    "delegation shard does not match canister",
                ));
            }

            $crate::__internal::core::api::auth::DelegationApi::store_proof(
                request,
                ::canic::dto::auth::DelegationProvisionTargetKind::Signer,
            )
            .await
        }

        #[cfg(canic_accepts_delegation_verifier_proof)]
        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_set_verifier_proof(
            request: ::canic::dto::auth::DelegationProofInstallRequest,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::store_proof(
                request,
                ::canic::dto::auth::DelegationProvisionTargetKind::Verifier,
            )
            .await
        }
    };
}

// Default shared endpoint surface for all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints {
    () => {
        $crate::canic_endpoints_lifecycle_core!();
        $crate::canic_endpoints_standards!();
        $crate::canic_endpoints_observability!();
        #[cfg(not(canic_disable_bundle_metrics))]
        $crate::canic_endpoints_metrics!();
        #[cfg(not(canic_disable_bundle_auth_attestation))]
        $crate::canic_endpoints_auth_attestation!();
        $crate::canic_endpoints_topology_views!();
    };
}

// Default root-only endpoint surface.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        $crate::canic_endpoints_root_admin!();
        $crate::canic_endpoints_root_auth_attestation!();
    };
}

// Default non-root-only endpoint surface.
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_endpoints_nonroot_sync_topology!();
        $crate::canic_endpoints_nonroot_auth_attestation!();
    };
}
