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
        fn canic_memory_registry() -> ::canic::core::ops::model::memory::registry::MemoryRegistryView {
            $crate::ops::model::memory::registry::MemoryRegistryOps::export()
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::ops::model::memory::env::EnvData {
            $crate::ops::model::memory::EnvOps::export()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::model::memory::log::LogPageDto {
            ::canic::core::ops::model::memory::log::LogOps::page(
                crate_name, topic, min_level, page,
            )
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> ::canic::core::ops::metrics::SystemMetricsSnapshot {
            ::canic::core::ops::metrics::MetricsOps::system_snapshot()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::metrics::MetricsPageDto<::canic::core::ops::metrics::IccMetricEntry>
        {
            ::canic::core::ops::metrics::MetricsOps::icc_page(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::metrics::MetricsPageDto<
            ::canic::core::ops::metrics::HttpMetricEntry,
        > {
            ::canic::core::ops::metrics::MetricsOps::http_page(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::metrics::MetricsPageDto<
            ::canic::core::ops::metrics::TimerMetricEntry,
        > {
            ::canic::core::ops::metrics::MetricsOps::timer_page(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::metrics::MetricsPageDto<
            ::canic::core::ops::metrics::AccessMetricEntry,
        > {
            ::canic::core::ops::metrics::MetricsOps::access_page(page)
        }

        #[canic_query]
        fn canic_perf(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::perf::PerfSnapshot {
            ::canic::core::ops::perf::PerfOps::snapshot(page)
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::ops::model::memory::state::AppStateData {
            $crate::ops::model::memory::state::AppStateOps::export()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::ops::model::memory::state::SubnetStateData {
            $crate::ops::model::memory::state::SubnetStateOps::export()
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::model::memory::directory::DirectoryPageDto {
            $crate::ops::model::memory::directory::AppDirectoryOps::page(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::types::PageRequest,
        ) -> Result<::canic::core::ops::model::memory::directory::DirectoryPageDto, ::canic::Error> {
            $crate::ops::model::memory::directory::SubnetDirectoryOps::page(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_subnet_canister_children(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::model::memory::topology::subnet::SubnetCanisterChildrenPage {
            ::canic::core::ops::model::memory::topology::subnet::SubnetCanisterChildrenOps::page(
                page,
            )
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::types::PageRequest,
        ) -> ::canic::core::ops::model::memory::cycles::CycleTrackerPage {
            $crate::ops::model::memory::cycles::CycleTrackerOps::page(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::ops::model::memory::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::ops::model::memory::scaling::ScalingRegistryOps::export())
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::auth::is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::core::ops::model::memory::sharding::ShardingRegistryDto, ::canic::Error> {
            Ok($crate::ops::model::memory::sharding::ShardingPolicyOps::export())
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
            use $crate::cdk::{
                api::canister_self,
                management_canister::{CanisterStatusArgs, canister_status},
            };

            if &msg_caller().to_string() != "ylse7-raaaa-aaaal-qsrsa-cai" {
                return Err(::canic::Error::custom("Unauthorized"));
            }

            canister_status(&CanisterStatusArgs {
                canister_id: canister_self(),
            })
            .await
            .map_err(|e| ::canic::Error::custom(e.to_string()))
        }
    };
}
