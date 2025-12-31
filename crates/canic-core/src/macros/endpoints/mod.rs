// Macros that generate public IC endpoints for Canic canisters.

pub mod root;

// Expose the shared query and update handlers used by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints {
    () => {
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
            $crate::workflow::endpoints::icrc10_supported_standards()
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::core::cdk::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::core::cdk::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::workflow::endpoints::icrc21_canister_call_consent_message(req)
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
            $crate::workflow::endpoints::canic_memory_registry()
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::dto::env::EnvView {
            $crate::workflow::endpoints::canic_env()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::log::LogEntryView> {
            $crate::workflow::endpoints::canic_log(crate_name, topic, min_level, page)
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> ::canic::core::ops::runtime::metrics::SystemMetricsSnapshot {
            $crate::workflow::endpoints::canic_metrics_system()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::IccMetricEntry> {
            $crate::workflow::endpoints::canic_metrics_icc(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::HttpMetricEntry> {
            $crate::workflow::endpoints::canic_metrics_http(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::TimerMetricEntry>
        {
            $crate::workflow::endpoints::canic_metrics_timer(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::AccessMetricEntry>
        {
            $crate::workflow::endpoints::canic_metrics_access(page)
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::perf::PerfEntry> {
            $crate::workflow::endpoints::canic_metrics_perf(page)
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::endpoint::EndpointHealthView> {
            $crate::workflow::endpoints::canic_metrics_endpoint_health(page)
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::dto::state::AppStateView {
            $crate::workflow::endpoints::canic_app_state()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::dto::state::SubnetStateView {
            $crate::workflow::endpoints::canic_subnet_state()
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
            $crate::workflow::endpoints::canic_app_directory(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(
            ::canic::core::ids::CanisterRole,
            ::canic::core::cdk::types::Principal,
        )> {
            $crate::workflow::endpoints::canic_subnet_directory(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_subnet_canister_children(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::canister::CanisterSummaryView> {
            $crate::workflow::endpoints::canic_subnet_canister_children(page)
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(u64, ::canic::core::cdk::types::Cycles)> {
            $crate::workflow::endpoints::canic_cycle_tracker(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::dto::placement::ScalingRegistryView, ::canic::PublicError> {
            $crate::workflow::endpoints::canic_scaling_registry()
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::core::dto::placement::ShardingRegistryView, ::canic::PublicError> {
            $crate::workflow::endpoints::canic_sharding_registry()
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //

        #[canic_query]
        fn icts_name() -> String {
            env!("CARGO_PKG_NAME").to_string()
        }

        #[canic_query]
        fn icts_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }

        #[canic_query]
        fn icts_description() -> String {
            env!("CARGO_PKG_DESCRIPTION").to_string()
        }

        #[canic_query]
        fn icts_metadata() -> Vec<(String, String)> {
            vec![
                ("name".to_string(), icts_name()),
                ("version".to_string(), icts_version()),
                ("description".to_string(), icts_description()),
            ]
        }

        /// ICTS add-on endpoint: returns string errors by design.
        #[canic_update]
        async fn icts_canister_status()
        -> Result<::canic::cdk::management_canister::CanisterStatusResult, String> {
            use $crate::cdk::api::msg_caller;

            static ICTS_CALLER: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text("ylse7-raaaa-aaaal-qsrsa-cai")
                        .expect("ICTS caller principal must be valid")
                });

            if msg_caller() != *ICTS_CALLER {
                return Err("unauthorized".to_string());
            }

            $crate::workflow::endpoints::icts_canister_status().await
        }
    };
}
