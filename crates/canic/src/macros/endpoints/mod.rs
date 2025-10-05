//! Macros that generate public IC endpoints for Canic canisters.

pub mod root;

/// Expose the shared query and update handlers used by all Canic canisters.
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
        // Canic CANISTER HELPERS
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
        // Canic MEMORY REGISTRY EXPORTS
        //

        #[::canic::cdk::query]
        fn canic_memory_registry() -> ::canic::memory::registry::MemoryRegistryView {
            $crate::memory::registry::MemoryRegistry::export()
        }

        //
        // Canic MEMORY CONTEXT EXPORTS
        //

        #[::canic::cdk::query]
        fn canic_canister_context() -> ::canic::memory::context::CanisterContextData {
            $crate::memory::context::CanisterContext::export()
        }

        //
        // Canic MEMORY STATE EXPORTS
        //

        #[::canic::cdk::query]
        fn canic_app_state() -> ::canic::memory::state::AppStateData {
            $crate::memory::state::AppState::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_state() -> ::canic::memory::state::SubnetStateData {
            $crate::memory::state::SubnetState::export()
        }

        #[::canic::cdk::query]
        fn canic_canister_state() -> ::canic::memory::state::CanisterStateData {
            $crate::memory::state::CanisterState::export()
        }

        //
        // Canic CAPABILITY ENDPOINTS
        //

        // canic_cycle_tracker
        // for all canisters right now, but it's part of capabilities
        #[::canic::cdk::query]
        fn canic_cycle_tracker() -> ::canic::memory::capability::cycles::CycleTrackerView {
            $crate::memory::capability::cycles::CycleTracker::export()
        }

        #[cfg(canic_capability_delegation)]
        $crate::canic_endpoints_delegation!();

        #[cfg(canic_capability_scaling)]
        $crate::canic_endpoints_scaling!();

        #[cfg(canic_capability_sharding)]
        $crate::canic_endpoints_sharding!();

        //
        // ICTS ENDPOINTS
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

/// Add delegation-specific endpoints when the capability is enabled.
#[macro_export]
macro_rules! canic_endpoints_delegation {
    () => {
        //
        // Canic DELEGATION ENDPOINTS
        //

        #[::canic::cdk::query]
        async fn canic_delegation_get(
            session_pid: ::candid::Principal,
        ) -> Result<::canic::state::delegation::DelegationSessionView, ::canic::Error> {
            $crate::ops::delegation::DelegationRegistry::get_view(session_pid)
        }

        #[::canic::cdk::update]
        async fn canic_delegation_track(
            session_pid: ::candid::Principal,
        ) -> Result<::canic::state::delegation::DelegationSessionView, ::canic::Error> {
            $crate::ops::delegation::DelegationRegistry::track(msg_caller(), session_pid)
        }

        #[::canic::cdk::update]
        async fn canic_delegation_register(
            args: ::canic::state::delegation::RegisterSessionArgs,
        ) -> Result<::canic::state::delegation::DelegationSessionView, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_whitelisted)?;

            let wallet = msg_caller();

            $crate::ops::delegation::DelegationRegistry::register_session(wallet, args.clone())?;
            $crate::ops::delegation::DelegationRegistry::get_view(args.session_pid)
        }

        #[::canic::cdk::update]
        async fn canic_delegation_revoke(pid: ::candid::Principal) -> Result<(), ::canic::Error> {
            use ::canic::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::ops::delegation::DelegationRegistry::revoke(pid)
        }

        // List all delegation sessions (admin only)
        #[::canic::cdk::query]
        async fn canic_delegation_list_all()
        -> Result<Vec<::canic::state::delegation::DelegationSessionView>, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            Ok($crate::ops::delegation::DelegationRegistry::list_all_sessions())
        }

        // List sessions by wallet (admin only)
        #[::canic::cdk::query]
        async fn canic_delegation_list_by_wallet(
            wallet_pid: ::candid::Principal,
        ) -> Result<Vec<::canic::state::delegation::DelegationSessionView>, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            Ok($crate::ops::delegation::DelegationRegistry::list_sessions_by_wallet(wallet_pid))
        }
    };
}

/// Add scaling registry endpoints when the capability is enabled.
#[macro_export]
macro_rules! canic_endpoints_scaling {
    () => {
        // canic_scaling_registry
        #[::canic::cdk::query]
        async fn canic_scaling_registry()
        -> Result<::canic::memory::capability::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::ops::scaling::export_registry())
        }
    };
}

/// Add sharding endpoints when the capability is enabled.
#[macro_export]
macro_rules! canic_endpoints_sharding {
    () => {
        // canic_sharding_registry
        #[::canic::cdk::query]
        async fn canic_sharding_registry()
        -> Result<::canic::memory::capability::sharding::ShardingRegistryView, ::canic::Error> {
            Ok($crate::ops::sharding::export_registry())
        }

        // canic_sharding_lookup_tenant
        // can be called by any principal
        #[::canic::cdk::query]
        async fn canic_sharding_lookup_tenant(
            pool: String,
            tenant_pid: ::candid::Principal,
        ) -> Result<::candid::Principal, ::canic::Error> {
            $crate::ops::sharding::try_lookup_tenant(&pool, tenant_pid)
        }

        // canic_sharding_admin
        // combined admin endpoint for shard lifecycle operations (controller only).
        #[::canic::cdk::update]
        async fn canic_sharding_admin(
            cmd: ::canic::ops::sharding::AdminCommand,
        ) -> Result<::canic::ops::sharding::AdminResult, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            $crate::ops::sharding::admin_command(cmd).await
        }
    };
}
