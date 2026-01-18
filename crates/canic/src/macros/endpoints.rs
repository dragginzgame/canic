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
        #[canic_update]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICRC ENDPOINTS
        //

        #[canic_query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }

        //
        // CANISTER HELPERS
        //

        #[canic_query]
        fn canic_canister_cycle_balance() -> u128 {
            $crate::cdk::api::canister_cycle_balance()
        }

        #[canic_query]
        fn canic_canister_version() -> u64 {
            $crate::cdk::api::canister_version()
        }

        #[canic_query]
        fn canic_time() -> u64 {
            $crate::cdk::api::time()
        }

        //
        // MEMORY
        //

        #[canic_query]
        fn canic_memory_registry() -> ::canic::dto::memory::MemoryRegistryView {
            $crate::__internal::core::api::memory::MemoryQuery::registry_view()
        }

        #[canic_query]
        fn canic_env() -> ::canic::dto::env::EnvView {
            $crate::__internal::core::api::env::EnvQuery::view()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::__internal::core::log::Level>,
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::log::LogEntryView> {
            $crate::__internal::core::api::log::LogQuery::page(crate_name, topic, min_level, page)
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> Vec<::canic::dto::metrics::SystemMetricEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::system_snapshot()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::metrics::IccMetricEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::icc_page(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::metrics::HttpMetricEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::http_page(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::metrics::TimerMetricEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::timer_page(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::metrics::AccessMetricEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::access_page(page)
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::__internal::core::perf::PerfEntry> {
            $crate::__internal::core::api::metrics::MetricsQuery::perf_page(page)
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::metrics::EndpointHealthView> {
            $crate::__internal::core::api::metrics::MetricsQuery::endpoint_health_page(
                page,
                Some($crate::__internal::core::protocol::CANIC_METRICS_ENDPOINT_HEALTH),
            )
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::dto::state::AppStateView {
            $crate::__internal::core::api::state::AppStateQuery::view()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::dto::state::SubnetStateView {
            $crate::__internal::core::api::state::SubnetStateQuery::view()
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryView> {
            $crate::__internal::core::api::topology::directory::AppDirectoryApi::page(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::topology::DirectoryEntryView> {
            $crate::__internal::core::api::topology::directory::SubnetDirectoryApi::page(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_canister_children(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::canister::CanisterRecordView> {
            $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page)
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> ::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntryView> {
            $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::dto::placement::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::scaling::ScalingApi::registry_view())
        }

        //
        // SHARDING
        //

        #[canic_query(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::dto::placement::sharding::ShardingRegistryView, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::registry_view())
        }

        #[canic_query(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_sharding_tenants(
            pool: String,
            shard_pid: ::canic::__internal::core::cdk::types::Principal,
        ) -> Result<::canic::dto::placement::sharding::ShardingTenantsView, ::canic::Error> {
            Ok($crate::__internal::core::api::placement::sharding::ShardingApi::tenants_view(&pool, shard_pid))
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //
        // NOTE: ICTS return types are fixed by a third-party standard; do not change them.

        #[canic_query]
        fn icts_name() -> String {
            $crate::__internal::core::api::icts::IctsApi::name()
        }

        #[canic_query]
        fn icts_version() -> String {
            $crate::__internal::core::api::icts::IctsApi::version()
        }

        #[canic_query]
        fn icts_description() -> String {
            $crate::__internal::core::api::icts::IctsApi::description()
        }

        #[canic_query]
        fn icts_metadata() -> ::canic::dto::icts::CanisterMetadataView {
            $crate::__internal::core::api::icts::IctsApi::metadata()
        }

        /// ICTS add-on endpoint: returns string errors by design.
        #[canic_update]
        async fn icts_canister_status()
        -> Result<::canic::dto::canister::CanisterStatusView, String> {
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
        #[canic_update(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_app(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::state::AppStateApi::execute_command(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(auth(::canic::dsl::access::auth::caller_is_controller))]
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
        #[canic_update(auth(::canic::dsl::access::auth::caller_is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::dto::rpc::Request,
        ) -> Result<::canic::dto::rpc::Response, ::canic::Error> {
            let response = $crate::__internal::core::api::rpc::RpcApi::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[canic_update(auth(
            ::canic::dsl::access::auth::caller_is_root,
            ::canic::dsl::access::auth::caller_is_controller
        ))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusView, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_app_registry() -> ::canic::dto::topology::AppRegistryView {
            $crate::__internal::core::api::topology::registry::AppRegistryApi::view()
        }

        #[canic_query]
        fn canic_subnet_registry() -> ::canic::dto::topology::SubnetRegistryView {
            $crate::__internal::core::api::topology::registry::SubnetRegistryApi::view()
        }

        //
        // CANISTER POOL
        //

        #[canic_query]
        async fn canic_pool_list() -> ::canic::dto::pool::CanisterPoolView {
            $crate::__internal::core::api::pool::CanisterPoolApi::list_view()
        }

        #[canic_update(auth(::canic::dsl::access::auth::caller_is_controller))]
        async fn canic_pool_admin(
            cmd: ::canic::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::dto::pool::PoolAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::pool::CanisterPoolApi::admin(cmd).await
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

        #[canic_update(auth(::canic::dsl::access::auth::caller_is_parent))]
        async fn canic_sync_state(
            snapshot: ::canic::dto::cascade::StateSnapshotView,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot).await
        }

        #[canic_update(auth(::canic::dsl::access::auth::caller_is_parent))]
        async fn canic_sync_topology(
            snapshot: ::canic::dto::cascade::TopologySnapshotView,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_topology(snapshot).await
        }
    };
}
