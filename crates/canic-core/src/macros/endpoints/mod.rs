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
            $crate::ops::icrc::Icrc10Ops::supported_standards()
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::core::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::core::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::ops::icrc::Icrc21Ops::consent_message(req)
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
        fn canic_memory_registry() -> ::canic::core::ops::storage::memory::MemoryRegistryView {
            $crate::ops::storage::memory::MemoryRegistryOps::export()
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::ops::storage::env::EnvData {
            $crate::ops::storage::env::EnvOps::export()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::log::LogEntryDto> {
            ::canic::core::ops::runtime::log::LogOps::page(crate_name, topic, min_level, page)
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> ::canic::core::ops::runtime::metrics::SystemMetricsSnapshot {
            ::canic::core::ops::runtime::metrics::MetricsOps::system_snapshot()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::IccMetricEntry> {
            ::canic::core::ops::runtime::metrics::MetricsOps::icc_page(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::HttpMetricEntry> {
            ::canic::core::ops::runtime::metrics::MetricsOps::http_page(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::TimerMetricEntry>
        {
            ::canic::core::ops::runtime::metrics::MetricsOps::timer_page(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::AccessMetricEntry>
        {
            ::canic::core::ops::runtime::metrics::MetricsOps::access_page(page)
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::perf::PerfEntry> {
            ::canic::core::ops::perf::PerfOps::snapshot(page)
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::runtime::metrics::EndpointHealthEntry> {
            ::canic::core::ops::runtime::metrics::MetricsOps::endpoint_health_page_excluding(
                page,
                Some(stringify!(canic_metrics_endpoint_health)),
            )
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::ops::storage::state::AppStateData {
            $crate::ops::storage::state::AppStateOps::export()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::ops::storage::state::SubnetStateData {
            $crate::ops::storage::state::SubnetStateOps::export()
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
            $crate::ops::storage::directory::AppDirectoryOps::page(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(
            ::canic::core::ids::CanisterRole,
            ::canic::core::cdk::types::Principal,
        )> {
            $crate::ops::storage::directory::SubnetDirectoryOps::page(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_subnet_canister_children(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::ops::storage::CanisterSummary> {
            ::canic::core::ops::storage::topology::subnet::SubnetCanisterChildrenOps::page(page)
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<(u64, ::canic::core::types::Cycles)> {
            $crate::ops::runtime::cycles::CycleTrackerOps::page(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::ops::placement::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::ops::placement::scaling::ScalingRegistryOps::export())
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::auth::is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::core::ops::placement::sharding::ShardingRegistryDto, ::canic::Error> {
            Ok($crate::ops::placement::sharding::ShardingPolicyOps::export())
        }

        //
        // ICTS
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

        #[canic_update]
        async fn icts_canister_status()
        -> Result<::canic::cdk::management_canister::CanisterStatusResult, ::canic::Error> {
            use $crate::cdk::api::{canister_self, msg_caller};
            use $crate::ops::ic::canister_status;

            static ICTS_CALLER: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text("ylse7-raaaa-aaaal-qsrsa-cai")
                        .expect("ICTS caller principal must be valid")
                });

            if msg_caller() != *ICTS_CALLER {
                return Err(::canic::Error::custom("Unauthorized"));
            }

            canister_status(canister_self()).await
        }
    };
}
