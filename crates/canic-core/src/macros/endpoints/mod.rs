// Macros that generate public IC endpoints for Canic canisters.

pub mod root;

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
            $crate::api::icrc::Icrc10Query::supported_standards()
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::api::icrc::Icrc21Query::consent_message(req)
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
        fn canic_memory_registry() -> ::canic::core::dto::memory::MemoryRegistryView {
            $crate::api::memory::MemoryQuery::registry_view()
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::dto::env::EnvView {
            $crate::api::env::EnvQuery::view()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::log::LogEntryView> {
            $crate::api::log::LogQuery::page(crate_name, topic, min_level, page)
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> Vec<::canic::core::dto::metrics::SystemMetricEntry> {
            $crate::api::metrics::MetricsQuery::system_snapshot()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::IccMetricEntry> {
            $crate::api::metrics::MetricsQuery::icc_page(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::HttpMetricEntry> {
            $crate::api::metrics::MetricsQuery::http_page(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::TimerMetricEntry> {
            $crate::api::metrics::MetricsQuery::timer_page(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::AccessMetricEntry> {
            $crate::api::metrics::MetricsQuery::access_page(page)
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::perf::PerfEntry> {
            $crate::api::metrics::MetricsQuery::perf_page(page)
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::EndpointHealthView> {
            $crate::api::metrics::MetricsQuery::endpoint_health_page(
                page,
                Some($crate::protocol::CANIC_METRICS_ENDPOINT_HEALTH),
            )
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::dto::state::AppStateView {
            $crate::api::state::AppStateQuery::view()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::dto::state::SubnetStateView {
            $crate::api::state::SubnetStateQuery::view()
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::topology::DirectoryEntryView> {
            $crate::api::topology::directory::AppDirectoryApi::page(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::topology::DirectoryEntryView> {
            $crate::api::topology::directory::SubnetDirectoryApi::page(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_canister_children(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::canister::CanisterRecordView> {
            $crate::api::topology::children::CanisterChildrenApi::page(page)
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::cycles::CycleTrackerEntryView> {
            $crate::api::cycles::CycleTrackerQuery::page(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::dto::placement::scaling::ScalingRegistryView, ::canic::PublicError>
        {
            Ok($crate::api::placement::scaling::ScalingApi::registry_view())
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_registry() -> Result<
            ::canic::core::dto::placement::sharding::ShardingRegistryView,
            ::canic::PublicError,
        > {
            Ok($crate::api::placement::sharding::ShardingApi::registry_view())
        }

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_tenants(
            pool: String,
            shard_pid: ::canic::core::cdk::types::Principal,
        ) -> Result<
            ::canic::core::dto::placement::sharding::ShardingTenantsView,
            ::canic::PublicError,
        > {
            Ok($crate::api::placement::sharding::ShardingApi::tenants_view(
                &pool, shard_pid,
            ))
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //
        // NOTE: ICTS return types are fixed by a third-party standard; do not change them.

        #[canic_query]
        fn icts_name() -> String {
            $crate::api::icts::IctsApi::name()
        }

        #[canic_query]
        fn icts_version() -> String {
            $crate::api::icts::IctsApi::version()
        }

        #[canic_query]
        fn icts_description() -> String {
            $crate::api::icts::IctsApi::description()
        }

        #[canic_query]
        fn icts_metadata() -> ::canic::core::dto::icts::CanisterMetadataView {
            $crate::api::icts::IctsApi::metadata()
        }

        /// ICTS add-on endpoint: returns string errors by design.
        #[canic_update]
        async fn icts_canister_status()
        -> Result<::canic::core::dto::canister::CanisterStatusView, String> {
            use $crate::cdk::api::msg_caller;

            static ICTS_CALLER: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text("ylse7-raaaa-aaaal-qsrsa-cai")
                        .expect("ICTS caller principal must be valid")
                });

            if msg_caller() != *ICTS_CALLER {
                return Err("unauthorized".to_string());
            }

            $crate::api::icts::IctsApi::canister_status()
                .await
                .map_err(|err| err.to_string())
        }
    };
}
