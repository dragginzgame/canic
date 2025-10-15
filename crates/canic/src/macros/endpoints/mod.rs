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
            $crate::state::icrc::Icrc10Registry::supported_standards()
        }

        #[::canic::cdk::query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::state::icrc::Icrc21Registry::consent_message(req)
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
        // MEMORY REGISTRY
        //

        #[::canic::cdk::query]
        fn canic_memory_registry() -> ::canic::memory::registry::MemoryRegistryView {
            $crate::memory::registry::MemoryRegistry::export()
        }

        //
        // MEMORY ENV
        //

        #[::canic::cdk::query]
        fn canic_env() -> ::canic::memory::env::EnvData {
            $crate::memory::Env::export()
        }

        //
        // MEMORY TOPOLOGY
        //

        #[::canic::cdk::query]
        fn canic_app_subnet_registry() -> ::canic::memory::topology::AppSubnetRegistryView {
            $crate::memory::topology::AppSubnetRegistry::export()
        }

        #[::canic::cdk::query]
        fn canic_app_canister_registry() -> ::canic::memory::topology::AppSubnetRegistryView {
            $crate::memory::topology::AppSubnetRegistry::export()
        }

        //
        // MEMORY DIRECTORY
        //

        #[::canic::cdk::query]
        fn canic_app_directory() -> ::canic::memory::directory::AppDirectoryView {
            $crate::memory::directory::AppDirectory::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_directory() -> ::canic::memory::directory::SubnetDirectoryView {
            $crate::memory::directory::SubnetDirectory::export()
        }

        //
        // STATE
        //

        #[::canic::cdk::query]
        fn canic_app_state() -> ::canic::memory::state::AppStateData {
            $crate::memory::state::AppState::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_state() -> ::canic::memory::state::SubnetStateData {
            $crate::memory::state::SubnetState::export()
        }

        //
        // CYCLES
        //

        // canic_cycle_tracker
        #[::canic::cdk::query]
        fn canic_cycle_tracker() -> ::canic::memory::ext::cycles::CycleTrackerView {
            $crate::memory::ext::cycles::CycleTracker::export()
        }

        //
        // SCALING
        //

        // canic_scaling_registry
        #[::canic::cdk::query]
        async fn canic_scaling_registry()
        -> Result<::canic::memory::ext::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::ops::ext::scaling::export_registry())
        }

        //
        // SHARDING
        //

        // canic_sharding_registry
        #[::canic::cdk::query]
        async fn canic_sharding_registry()
        -> Result<::canic::memory::ext::sharding::ShardingRegistryView, ::canic::Error> {
            Ok($crate::ops::ext::sharding::export_registry())
        }

        // canic_sharding_lookup_tenant
        // can be called by any principal
        #[::canic::cdk::query]
        async fn canic_sharding_lookup_tenant(
            pool: String,
            tenant_pid: ::candid::Principal,
        ) -> Result<::candid::Principal, ::canic::Error> {
            $crate::ops::ext::sharding::try_lookup_tenant(&pool, tenant_pid)
        }

        // canic_sharding_admin
        // combined admin endpoint for shard lifecycle operations (controller only).
        #[::canic::cdk::update]
        async fn canic_sharding_admin(
            cmd: ::canic::ops::ext::sharding::AdminCommand,
        ) -> Result<::canic::ops::ext::sharding::AdminResult, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            $crate::ops::ext::sharding::admin_command(cmd).await
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
