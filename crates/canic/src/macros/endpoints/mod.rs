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
        #[::canic::cdk::update]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICRC ENDPOINTS
        //

        #[::canic::cdk::query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::model::icrc::Icrc10Registry::supported_standards()
        }

        #[::canic::cdk::query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::model::icrc::Icrc21Registry::consent_message(req)
        }

        //
        // CANISTER HELPERS
        //

        #[::canic::cdk::query]
        fn canic_canister_cycle_balance() -> u128 {
            $crate::cdk::api::canister_cycle_balance()
        }

        #[::canic::cdk::query]
        fn canic_canister_version() -> u64 {
            $crate::cdk::api::canister_version()
        }

        #[::canic::cdk::query]
        fn canic_time() -> u64 {
            $crate::cdk::api::time()
        }

        //
        // MEMORY
        //

        #[::canic::cdk::query]
        fn canic_memory_registry() -> ::canic::ops::model::memory::registry::MemoryRegistryView {
            $crate::ops::model::memory::registry::MemoryRegistry::export()
        }

        #[::canic::cdk::query]
        fn canic_env() -> ::canic::ops::model::memory::env::EnvData {
            $crate::ops::model::memory::EnvOps::export()
        }

        #[::canic::cdk::query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::log::Level>,
            offset: u64,
            limit: u64,
        ) -> ::canic::ops::model::memory::log::LogPageDto {
            ::canic::ops::model::memory::log::LogOps::page(
                crate_name, topic, min_level, offset, limit,
            )
        }

        //
        // STATE
        //

        #[::canic::cdk::query]
        fn canic_app_state() -> ::canic::model::memory::state::AppStateData {
            $crate::ops::model::memory::state::AppStateOps::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_state() -> ::canic::model::memory::state::SubnetStateData {
            $crate::ops::model::memory::state::SubnetStateOps::export()
        }

        //
        // DIRECTORY VIEWS
        //

        #[::canic::cdk::query]
        fn canic_app_directory(
            offset: u64,
            limit: u64,
        ) -> ::canic::ops::model::memory::directory::DirectoryPageDto {
            $crate::ops::model::memory::directory::AppDirectoryOps::page(offset, limit)
        }

        #[::canic::cdk::query]
        fn canic_subnet_directory(
            offset: u64,
            limit: u64,
        ) -> Result<::canic::ops::model::memory::directory::DirectoryPageDto, ::canic::Error> {
            $crate::ops::model::memory::directory::SubnetDirectoryOps::page(offset, limit)
        }

        //
        // TOPOLOGY
        //

        #[::canic::cdk::query]
        fn canic_subnet_canister_children(
            offset: u64,
            limit: u64,
        ) -> ::canic::ops::model::memory::topology::subnet::CanisterChildrenPage {
            ::canic::ops::model::memory::topology::subnet::CanisterChildrenOps::page(offset, limit)
        }

        //
        // CYCLES
        //

        // canic_cycle_tracker
        #[::canic::cdk::query]
        fn canic_cycle_tracker(
            offset: u64,
            limit: u64,
        ) -> ::canic::ops::model::memory::cycles::CycleTrackerPage {
            $crate::ops::model::memory::cycles::CycleTrackerOps::page(offset, limit)
        }

        //
        // SCALING
        //

        // canic_scaling_registry
        #[::canic::cdk::query]
        async fn canic_scaling_registry()
        -> Result<::canic::model::memory::scaling::ScalingRegistryView, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            Ok($crate::ops::model::memory::scaling::export_registry())
        }

        //
        // SHARDING
        //

        // canic_sharding_registry
        #[::canic::cdk::query]
        async fn canic_sharding_registry()
        -> Result<::canic::model::memory::sharding::ShardingRegistryView, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            Ok($crate::ops::model::memory::sharding::ShardingPolicyOps::export_registry())
        }

        //
        // ICTS
        //

        #[::canic::cdk::query]
        fn icts_name() -> String {
            env!("CARGO_PKG_NAME").to_string()
        }

        #[::canic::cdk::query]
        fn icts_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }

        #[::canic::cdk::query]
        fn icts_description() -> String {
            env!("CARGO_PKG_DESCRIPTION").to_string()
        }

        #[::canic::cdk::query]
        fn icts_metadata() -> Vec<(String, String)> {
            vec![
                ("name".to_string(), icts_name()),
                ("version".to_string(), icts_version()),
                ("description".to_string(), icts_description()),
            ]
        }

        #[::canic::cdk::update]
        async fn icts_canister_status()
        -> Result<::canic::cdk::management_canister::CanisterStatusResult, String> {
            use $crate::cdk::{
                api::canister_self,
                management_canister::{CanisterStatusArgs, canister_status},
            };

            if &msg_caller().to_string() != "ylse7-raaaa-aaaal-qsrsa-cai" {
                return Err("Unauthorized".to_string());
            }

            canister_status(&CanisterStatusArgs {
                canister_id: canister_self(),
            })
            .await
            .map_err(|e| e.to_string())
        }
    };
}
