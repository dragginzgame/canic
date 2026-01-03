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
            $crate::api::endpoints::icrc10_supported_standards().expect("fix me")
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::core::cdk::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::core::cdk::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::api::endpoints::icrc21_canister_call_consent_message(req).expect("fix me")
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
            $crate::api::endpoints::canic_memory_registry().expect("fix me")
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::dto::env::EnvView {
            $crate::api::endpoints::canic_env().expect("fix me")
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::log::LogEntryView> {
            $crate::api::endpoints::canic_log(crate_name, topic, min_level, page).expect("fix me")
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> Vec<::canic::core::dto::metrics::SystemMetricEntry> {
            $crate::api::endpoints::canic_metrics_system().expect("fix me")
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::IccMetricEntry> {
            $crate::api::endpoints::canic_metrics_icc(page).expect("fix me")
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::HttpMetricEntry> {
            $crate::api::endpoints::canic_metrics_http(page).expect("fix me")
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::TimerMetricEntry> {
            $crate::api::endpoints::canic_metrics_timer(page).expect("fix me")
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::AccessMetricEntry> {
            $crate::api::endpoints::canic_metrics_access(page).expect("fix me")
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::perf::PerfEntry> {
            $crate::api::endpoints::canic_metrics_perf(page).expect("fix me")
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::EndpointHealthView> {
            $crate::api::endpoints::canic_metrics_endpoint_health(page).expect("fix me")
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::dto::state::AppStateView {
            $crate::api::endpoints::canic_app_state().expect("fix me")
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::dto::state::SubnetStateView {
            $crate::api::endpoints::canic_subnet_state().expect("fix me")
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(
            ::canic::core::ids::CanisterRole,
            ::canic::core::cdk::types::Principal,
        )> {
            $crate::api::endpoints::canic_app_directory(page).expect("fix me")
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(
            ::canic::core::ids::CanisterRole,
            ::canic::core::cdk::types::Principal,
        )> {
            $crate::api::endpoints::canic_subnet_directory(page).expect("fix me")
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_canister_children(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::canister::CanisterSummaryView> {
            $crate::api::endpoints::canic_canister_children(page).expect("fix me")
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(u64, ::canic::core::cdk::types::Cycles)> {
            $crate::api::endpoints::canic_cycle_tracker(page).expect("fix me")
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::dto::placement::ScalingRegistryView, ::canic::PublicError> {
            $crate::api::endpoints::canic_scaling_registry()
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::core::dto::placement::ShardingRegistryView, ::canic::PublicError> {
            $crate::api::endpoints::canic_sharding_registry()
        }

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_tenants(
            pool: String,
            shard_pid: ::canic::core::cdk::types::Principal,
        ) -> Result<::canic::core::dto::placement::ShardingTenantsView, ::canic::PublicError> {
            $crate::api::endpoints::canic_sharding_tenants(pool, shard_pid)
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //
        // NOTE: ICTS return types are fixed by a third-party standard; do not change them.

        #[canic_query]
        fn icts_name() -> String {
            $crate::api::endpoints::icts::icts_name()
        }

        #[canic_query]
        fn icts_version() -> String {
            $crate::api::endpoints::icts::icts_version()
        }

        #[canic_query]
        fn icts_description() -> String {
            $crate::api::endpoints::icts::icts_description()
        }

        #[canic_query]
        fn icts_metadata() -> ::canic::core::dto::canister::CanisterMetadataView {
            $crate::api::endpoints::icts::icts_metadata()
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

            $crate::api::endpoints::icts::icts_canister_status().await
        }
    };
}
