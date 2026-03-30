// -----------------------------------------------------------------------------
// Endpoint bundle macros
// -----------------------------------------------------------------------------

// Macros that generate public IC endpoints for Canic canisters.
// These emitters and bundles define the compile-time capability surface for
// `start!` and `start_root!`. The default compositions intentionally preserve
// the current feature set; bundle boundaries exist to make linker policy
// explicit.

// -----------------------------------------------------------------------------
// Leaf endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the lifecycle/runtime core shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_lifecycle_core_endpoints {
    () => {
        #[canic_update(internal)]
        fn canic_ic_cycles_accept(max_amount: u128) -> u128 {
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

// Leaf emitter for the ICRC standards-facing surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_icrc_standards_endpoints {
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

// Leaf emitter for Canic metadata shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_canic_metadata_endpoints {
    () => {
        #[canic_query(internal)]
        fn canic_standards() -> ::canic::dto::standards::CanicStandardsResponse {
            $crate::__internal::core::api::standards::CanicStandardsApi::metadata()
        }
    };
}

// Bundle composer for the standards-facing surface preserved by the default runtime.
#[macro_export]
macro_rules! canic_bundle_standards_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_standards_icrc))]
        $crate::canic_emit_icrc_standards_endpoints!();
        #[cfg(not(canic_disable_bundle_standards_canic))]
        $crate::canic_emit_canic_metadata_endpoints!();
    };
}

// Leaf emitter for runtime memory-registry diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_memory_observability_endpoints {
    () => {
        #[canic_query]
        fn canic_memory_registry()
        -> Result<::canic::dto::memory::MemoryRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::memory::MemoryQuery::registry())
        }
    };
}

// Leaf emitter for environment snapshot diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_env_observability_endpoints {
    () => {
        #[canic_query]
        fn canic_env() -> Result<::canic::dto::env::EnvSnapshotResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::env::EnvQuery::snapshot())
        }
    };
}

// Leaf emitter for runtime log diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_log_observability_endpoints {
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

// Bundle composer for shared observability and operator-facing diagnostics.
#[macro_export]
macro_rules! canic_bundle_observability_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_observability_memory))]
        $crate::canic_emit_memory_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_env))]
        $crate::canic_emit_env_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_log))]
        $crate::canic_emit_log_observability_endpoints!();
    };
}

// Leaf emitter for the metrics query surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_metrics_endpoints {
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

// Leaf emitter for the response-capability and trust-chain runtime.
#[macro_export]
macro_rules! canic_emit_auth_attestation_endpoints {
    () => {
        #[cfg(canic_is_root)]
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::RootCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::RootCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_root(envelope).await
        }

        #[cfg(not(canic_is_root))]
        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::RootCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::RootCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(envelope)
                .await
        }
    };
}

// Leaf emitter for shared state snapshots.
#[macro_export]
macro_rules! canic_emit_topology_state_endpoints {
    () => {
        #[cfg(canic_is_root)]
        #[canic_query]
        fn canic_app_state() -> Result<::canic::dto::state::AppStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::AppStateQuery::snapshot())
        }

        #[cfg(canic_is_root)]
        #[canic_query]
        fn canic_subnet_state() -> Result<::canic::dto::state::SubnetStateResponse, ::canic::Error>
        {
            Ok($crate::__internal::core::api::state::SubnetStateQuery::snapshot())
        }
    };
}

// Leaf emitter for shared directory views.
#[macro_export]
macro_rules! canic_emit_topology_directory_endpoints {
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

// Leaf emitter for the shared topology-children view.
#[macro_export]
macro_rules! canic_emit_topology_children_endpoints {
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

// Leaf emitter for the shared cycle-tracker view.
#[macro_export]
macro_rules! canic_emit_topology_cycles_endpoints {
    () => {
        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }
    };
}

// Leaf emitter for shared scaling/sharding placement views.
#[macro_export]
macro_rules! canic_emit_topology_placement_endpoints {
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
            Ok(::canic_sharding_runtime::api::ShardingApi::registry())
        }

        #[cfg(canic_has_sharding)]
        #[canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_partition_keys(
            pool: String,
            shard_pid: ::canic::__internal::core::cdk::types::Principal,
        ) -> Result<::canic::dto::placement::sharding::ShardingPartitionKeysResponse, ::canic::Error> {
            Ok(::canic_sharding_runtime::api::ShardingApi::partition_keys(&pool, shard_pid))
        }
    };
}

// Bundle composer for shared state, directory, topology, and placement views.
#[macro_export]
macro_rules! canic_bundle_topology_views_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_topology_state))]
        $crate::canic_emit_topology_state_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_directory))]
        $crate::canic_emit_topology_directory_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_children))]
        $crate::canic_emit_topology_children_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_cycles))]
        $crate::canic_emit_topology_cycles_endpoints!();
        #[cfg(not(canic_disable_bundle_topology_placement))]
        $crate::canic_emit_topology_placement_endpoints!();
    };
}

// Leaf emitter for the root-only control-plane, registry, and operator admin surface.
#[macro_export]
macro_rules! canic_emit_root_admin_endpoints {
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

// Leaf emitter for the root-only auth, delegation, and attestation authority surface.
#[macro_export]
macro_rules! canic_emit_root_auth_attestation_endpoints {
    () => {
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_delegation(
            request: ::canic::dto::auth::DelegationRequest,
        ) -> Result<::canic::dto::auth::DelegationProvisionResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::request_delegation_root(request)
                .await
        }

        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_role_attestation(
            request: ::canic::dto::auth::RoleAttestationRequest,
        ) -> Result<::canic::dto::auth::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::request_role_attestation_root(
                request,
            )
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

// Leaf emitter for the root-only WasmStore bootstrap/publication control surface.
#[macro_export]
macro_rules! canic_emit_root_wasm_store_endpoints {
    () => {
        #[canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_stage_manifest_admin(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::stage_root_wasm_store_manifest(
                request,
            )
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_prepare_admin(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_root_wasm_store_chunk_set(request)
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_publish_chunk_admin(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_root_wasm_store_chunk(request)
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_resume_root_admin() -> Result<(), ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_init_root_bootstrap();
            Ok(())
        }

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_debug(
        ) -> Result<::canic::dto::template::WasmStoreBootstrapDebugResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::debug_bootstrap()
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_template_stage_manifest_admin(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::stage_manifest(request);
            Ok(())
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_template_prepare_admin(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_chunk_set(request)
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_template_publish_chunk_admin(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_chunk(request)
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_template_publish_to_current_store_admin() -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_staged_release_set_to_current_store().await
        }

        #[canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_admin(
            cmd: ::canic::dto::template::WasmStoreAdminCommand,
        ) -> Result<::canic::dto::template::WasmStoreAdminResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::admin(cmd).await
        }

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_overview(
        ) -> Result<::canic::dto::template::WasmStoreOverviewResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::overview()
        }

    };
}

// Leaf emitter for the non-root sync surface used for state/topology propagation.
#[macro_export]
macro_rules! canic_emit_nonroot_sync_topology_endpoints {
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

// Leaf emitter for the non-root auth/attestation provisioning surface.
#[macro_export]
macro_rules! canic_emit_nonroot_auth_attestation_endpoints {
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

// -----------------------------------------------------------------------------
// Bundle composers
// -----------------------------------------------------------------------------

// Bundle composer for the default shared runtime surface on all Canic canisters.
#[macro_export]
macro_rules! canic_bundle_shared_runtime_endpoints {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
        $crate::canic_bundle_standards_endpoints!();
        $crate::canic_bundle_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_metrics))]
        $crate::canic_emit_metrics_endpoints!();
        #[cfg(not(canic_disable_bundle_auth_attestation))]
        $crate::canic_emit_auth_attestation_endpoints!();
        $crate::canic_bundle_topology_views_endpoints!();
    };
}

// Bundle composer for the root-only runtime surface.
#[macro_export]
macro_rules! canic_bundle_root_only_endpoints {
    () => {
        $crate::canic_emit_root_admin_endpoints!();
        $crate::canic_emit_root_auth_attestation_endpoints!();
        $crate::canic_emit_root_wasm_store_endpoints!();
    };
}

// Bundle composer for the non-root-only runtime surface.
#[macro_export]
macro_rules! canic_bundle_nonroot_only_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
    };
}

// -----------------------------------------------------------------------------
// Backwards-compatible exported aliases
// -----------------------------------------------------------------------------

// Preserve the previous macro names for downstream crates while the clearer
// emit_/bundle_ names become the primary surface.
#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_lifecycle_core {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_standards_icrc {
    () => {
        $crate::canic_emit_icrc_standards_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_standards_canic {
    () => {
        $crate::canic_emit_canic_metadata_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_standards {
    () => {
        $crate::canic_bundle_standards_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_observability_memory {
    () => {
        $crate::canic_emit_memory_observability_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_observability_env {
    () => {
        $crate::canic_emit_env_observability_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_observability_log {
    () => {
        $crate::canic_emit_log_observability_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_observability {
    () => {
        $crate::canic_bundle_observability_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_metrics {
    () => {
        $crate::canic_emit_metrics_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_auth_attestation {
    () => {
        $crate::canic_emit_auth_attestation_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_state {
    () => {
        $crate::canic_emit_topology_state_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_directory {
    () => {
        $crate::canic_emit_topology_directory_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_children {
    () => {
        $crate::canic_emit_topology_children_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_cycles {
    () => {
        $crate::canic_emit_topology_cycles_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_placement {
    () => {
        $crate::canic_emit_topology_placement_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_topology_views {
    () => {
        $crate::canic_bundle_topology_views_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_root_admin {
    () => {
        $crate::canic_emit_root_admin_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_root_auth_attestation {
    () => {
        $crate::canic_emit_root_auth_attestation_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_root_wasm_store {
    () => {
        $crate::canic_emit_root_wasm_store_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_nonroot_sync_topology {
    () => {
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_nonroot_auth_attestation {
    () => {
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints {
    () => {
        $crate::canic_bundle_shared_runtime_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        $crate::canic_bundle_root_only_endpoints!();
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        $crate::canic_bundle_nonroot_only_endpoints!();
    };
}
