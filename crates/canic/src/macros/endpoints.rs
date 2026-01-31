// -----------------------------------------------------------------------------
// Endpoint bundle macros
// -----------------------------------------------------------------------------

// Macros that generate public IC endpoints for Canic canisters.

// Expose the shared query and update handlers used by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints {
    () => {
        // NOTE: Avoid `$crate` in endpoint signatures (args/returns); Candid rejects it.
        //
        // IC API ENDPOINTS (IMPORTANT!!)
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[canic_update(internal)]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICRC ENDPOINTS
        //

        #[canic_query(internal)]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[canic_query(internal)]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }

        //
        // CANISTER HELPERS
        //

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

        //
        // MEMORY
        //

        #[canic_query]
        fn canic_memory_registry() -> Result<::canic::dto::memory::MemoryRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::memory::MemoryQuery::registry())
        }

        #[canic_query]
        fn canic_env() -> Result<::canic::dto::env::EnvSnapshotResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::env::EnvQuery::snapshot())
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::__internal::core::log::Level>,
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::log::LogEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::log::LogQuery::page(crate_name, topic, min_level, page))
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> Result<Vec<::canic::dto::metrics::SystemMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::system_snapshot())
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::IccMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::icc_page(page))
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::HttpMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::http_page(page))
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::TimerMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::timer_page(page))
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::AccessMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::access_page(page))
        }

        #[canic_query]
        fn canic_metrics_delegation(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::DelegationMetricEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::delegation_page(page))
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::__internal::core::perf::PerfEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::perf_page(page))
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::EndpointHealth>, ::canic::Error> {
            Ok($crate::__internal::core::api::metrics::MetricsQuery::endpoint_health_page(
                page,
                Some($crate::__internal::core::protocol::CANIC_METRICS_ENDPOINT_HEALTH),
            ))
        }

        //
        // STATE
        //

        // Internal readiness barrier for bootstrap synchronization.
        #[canic_query(internal)]
        fn canic_ready() -> bool {
            $crate::__internal::core::api::ready::ReadyApi::is_ready()
        }

        #[canic_query]
        fn canic_app_state() -> Result<::canic::dto::state::AppStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::AppStateQuery::snapshot())
        }

        #[canic_query]
        fn canic_subnet_state() -> Result<::canic::dto::state::SubnetStateResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::state::SubnetStateQuery::snapshot())
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryResponse>, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::directory::AppDirectoryApi::page(page))
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryResponse>, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::directory::SubnetDirectoryApi::page(page))
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_canister_children(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }

        //
        // SCALING
        //

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_scaling_registry()
        -> Result<::canic::dto::placement::scaling::ScalingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::scaling::ScalingApi::registry())
        }

        //
        // SHARDING
        //

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_registry()
        -> Result<::canic::dto::placement::sharding::ShardingRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::registry())
        }

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_sharding_tenants(
            pool: String,
            shard_pid: ::canic::__internal::core::cdk::types::Principal,
        ) -> Result<::canic::dto::placement::sharding::ShardingTenantsResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::tenants(&pool, shard_pid))
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //
        // NOTE: ICTS return types are fixed by a third-party standard; do not change them.

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

        /// ICTS add-on endpoint: returns string errors by design.
        #[canic_update(internal)]
        async fn icts_canister_status()
        -> Result<::canic::dto::canister::CanisterStatusResponse, String> {
            use $crate::cdk::api::msg_caller;

            static ICTS_CALLER: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text("ylse7-raaaa-aaaal-qsrsa-cai")
                        .expect("ICTS caller principal must be valid")
                });

            if msg_caller() != *ICTS_CALLER {
                return Err("unauthorized".to_string());
            }

            $crate::__internal::core::api::icts::IctsApi::canister_status()
                .await
                .map_err(|err| err.to_string())
        }
    };
}

// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // root-only app-level state mutation endpoint
        #[canic_update(internal, requires(caller::is_controller()))]
        async fn canic_app(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::state::AppStateApi::execute_command(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(requires(caller::is_controller()))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::dto::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res =
                $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid)
                    .await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_response(
            request: ::canic::dto::rpc::Request,
        ) -> Result<::canic::dto::rpc::Response, ::canic::Error> {
            let response = $crate::__internal::core::api::rpc::RpcApi::response(request).await?;

            Ok(response)
        }

        // canic_response_authenticated
        // root's way to respond to a request that includes delegated auth
        // has to come from a direct child canister
        #[canic_update(
            internal,
            requires(caller::is_registered_to_subnet(), auth::authenticated())
        )]
        async fn canic_response_authenticated(
            request: ::canic::dto::rpc::AuthenticatedRequest,
        ) -> Result<::canic::dto::rpc::AuthenticatedResponse, ::canic::Error> {
            let response =
                $crate::__internal::core::api::rpc::RpcApi::response(request.request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[canic_update(requires(caller::is_root(), caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        //
        // REGISTRIES
        //

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

        //
        // CANISTER POOL
        //

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

        //
        // DELEGATION
        //

        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_admin(
            cmd: ::canic::dto::auth::DelegationAdminCommand,
        ) -> Result<::canic::dto::auth::DelegationAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationAdminApi::admin(cmd).await
        }

        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_provision(
            request: ::canic::dto::auth::DelegationProvisionRequest,
        ) -> Result<::canic::dto::auth::DelegationProvisionResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::provision(request).await
        }

        #[canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_delegation(
            request: ::canic::dto::auth::DelegationRequest,
        ) -> Result<::canic::dto::auth::DelegationProvisionResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::request_delegation(request).await
        }

        #[canic_query(internal, requires(caller::is_root()))]
        async fn canic_delegation_status()
        -> Result<::canic::dto::auth::DelegationStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::status()
        }

        //
        // SHARDING
        //

        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_sharding_admin(
            cmd: ::canic::dto::placement::sharding::ShardingAdminCommand,
        ) -> Result<::canic::dto::placement::sharding::ShardingAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::placement::sharding::ShardingAdminApi::admin(cmd).await
        }
    };
}

// Generate the endpoint surface for non-root canisters.
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        //
        // SYNC
        //

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

        //
        // DELEGATION
        //

        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_set_signer_proof(
            proof: ::canic::dto::auth::DelegationProof,
        ) -> Result<(), ::canic::Error> {
            let self_pid = $crate::__internal::core::cdk::api::canister_self();
            if proof.cert.signer_pid != self_pid {
                return Err(::canic::Error::invalid(
                    "delegation signer does not match canister",
                ));
            }

            $crate::__internal::core::api::auth::DelegationApi::store_proof(
                proof,
                ::canic::dto::auth::DelegationProvisionTargetKind::Signer,
            )
        }

        #[canic_update(internal, requires(caller::is_root()))]
        async fn canic_delegation_set_verifier_proof(
            proof: ::canic::dto::auth::DelegationProof,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::auth::DelegationApi::store_proof(
                proof,
                ::canic::dto::auth::DelegationProvisionTargetKind::Verifier,
            )
        }
    };
}
