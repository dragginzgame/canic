pub mod root;

// icu_endpoints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        //
        // IC API ENDPOINTS (IMPORTANT!!)
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[::icu::cdk::update]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICRC ENDPOINTS
        //

        #[::icu::cdk::query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::state::icrc::Icrc10Registry::supported_standards()
        }

        #[::icu::cdk::query]
        async fn icrc21_canister_call_consent_message(
            req: ::icu::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::icu::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::state::icrc::Icrc21Registry::consent_message(req)
        }

        //
        // ICU CANISTER HELPERS
        //

        #[::icu::cdk::query]
        fn icu_canister_cycle_balance() -> u128 {
            $crate::cdk::api::canister_cycle_balance()
        }

        #[::icu::cdk::query]
        fn icu_canister_version() -> u64 {
            $crate::cdk::api::canister_version()
        }

        #[::icu::cdk::query]
        fn icu_time() -> u64 {
            $crate::cdk::api::time()
        }

        //
        // ICU MEMORY REGISTRY EXPORTS
        //

        #[::icu::cdk::query]
        fn icu_memory_registry() -> ::icu::memory::registry::MemoryRegistryView {
            $crate::memory::registry::MemoryRegistry::export()
        }

        //
        // ICU MEMORY CANISTER ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icu_cycle_tracker() -> ::icu::memory::canister::CycleTrackerView {
            $crate::memory::canister::CycleTracker::export()
        }

        //
        // ICU MEMORY STATE EXPORTS
        //

        #[::icu::cdk::query]
        fn icu_app_state() -> ::icu::memory::state::AppStateData {
            $crate::memory::state::AppState::export()
        }

        #[::icu::cdk::query]
        fn icu_subnet_state() -> ::icu::memory::state::SubnetStateData {
            $crate::memory::state::SubnetState::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_state() -> ::icu::memory::state::CanisterStateData {
            $crate::memory::state::CanisterState::export()
        }

        //
        // ICTS ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icts_name() -> String {
            env!("CARGO_PKG_NAME").to_string()
        }

        #[::icu::cdk::query]
        fn icts_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }

        #[::icu::cdk::query]
        fn icts_description() -> String {
            env!("CARGO_PKG_DESCRIPTION").to_string()
        }

        #[::icu::cdk::query]
        fn icts_metadata() -> Vec<(String, String)> {
            vec![
                ("name".to_string(), icts_name()),
                ("version".to_string(), icts_version()),
                ("description".to_string(), icts_description()),
            ]
        }

        #[::icu::cdk::update]
        async fn icts_canister_status()
        -> Result<::icu::cdk::management_canister::CanisterStatusResult, String> {
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

        //
        // CFG-GATED ENDPOINTS
        //

        #[cfg(icu_capability_delegation)]
        $crate::icu_endpoints_delegation!();

        #[cfg(icu_capability_sharder)]
        $crate::icu_endpoints_shard!();
    };
}

// icu_endpoints_delegation
#[macro_export]
macro_rules! icu_endpoints_delegation {
    () => {
        //
        // ICU DELEGATION ENDPOINTS
        //

        #[::icu::cdk::query]
        async fn icu_delegation_get(
            session_pid: ::candid::Principal,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::ops::delegation::DelegationRegistry::get_view(session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_track(
            session_pid: ::candid::Principal,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::ops::delegation::DelegationRegistry::track(msg_caller(), session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_register(
            args: ::icu::state::delegation::RegisterSessionArgs,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_whitelisted)?;

            let wallet = msg_caller();

            $crate::ops::delegation::DelegationRegistry::register_session(wallet, args.clone())?;
            $crate::ops::delegation::DelegationRegistry::get_view(args.session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_revoke(pid: ::candid::Principal) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::ops::delegation::DelegationRegistry::revoke(pid)
        }

        // List all delegation sessions (admin only)
        #[::icu::cdk::query]
        async fn icu_delegation_list_all()
        -> Result<Vec<::icu::state::delegation::DelegationSessionView>, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            Ok($crate::ops::delegation::DelegationRegistry::list_all_sessions())
        }

        // List sessions by wallet (admin only)
        #[::icu::cdk::query]
        async fn icu_delegation_list_by_wallet(
            wallet_pid: ::candid::Principal,
        ) -> Result<Vec<::icu::state::delegation::DelegationSessionView>, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            Ok($crate::ops::delegation::DelegationRegistry::list_sessions_by_wallet(wallet_pid))
        }
    };
}

// icu_endpoints_shard
#[macro_export]
macro_rules! icu_endpoints_shard {
    () => {
        //
        // ICU SHARD ENDPOINTS
        //

        // icu_shard_registry
        #[::icu::cdk::query]
        async fn icu_shard_registry()
        -> Result<::icu::memory::shard::ShardRegistryView, ::icu::Error> {
            Ok($crate::ops::shard::export_registry())
        }

        // icu_shard_lookup
        // can be called by any principal
        #[::icu::cdk::query]
        async fn icu_shard_lookup(
            pool: String,
            tenant_pid: ::candid::Principal,
        ) -> Result<Option<::candid::Principal>, ::icu::Error> {
            Ok($crate::ops::shard::lookup_tenant(&pool, tenant_pid))
        }

        // icu_shard_admin
        // combined admin endpoint for shard lifecycle operations (controller only).
        #[::icu::cdk::update]
        async fn icu_shard_admin(
            cmd: ::icu::ops::shard::AdminCommand,
        ) -> Result<::icu::ops::shard::AdminResult, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            $crate::ops::shard::admin_command(cmd).await
        }
    };
}
