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
        #[::icu::cdk::query]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICU ENDPOINTS
        //

        #[::icu::cdk::update]
        async fn icu_state_update(
            bundle: ::icu::ops::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::state::save_state(&bundle);

            Ok(())
        }

        #[::icu::cdk::update]
        async fn icu_state_cascade(
            bundle: ::icu::ops::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::state::save_state(&bundle);
            $crate::ops::state::cascade(&bundle).await
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
        // ICU MEMORY EXPORTS
        //

        #[::icu::cdk::query]
        fn icu_app_state() -> ::icu::memory::AppStateData {
            $crate::memory::AppState::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_children() -> ::icu::memory::CanisterChildrenView {
            $crate::memory::CanisterChildren::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_state() -> ::icu::memory::CanisterStateData {
            $crate::memory::CanisterState::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_directory() -> ::icu::memory::CanisterDirectoryView {
            $crate::memory::CanisterDirectory::current_view()
        }

        #[::icu::cdk::query]
        fn icu_cycle_tracker() -> ::icu::memory::CycleTrackerView {
            $crate::memory::CycleTracker::export()
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
            $crate::state::delegation::DelegationRegistry::get(session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_track(
            session_pid: ::candid::Principal,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::state::delegation::DelegationRegistry::track(msg_caller(), session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_register(
            args: ::icu::state::delegation::RegisterSessionArgs,
        ) -> ::icu::state::delegation::DelegationSessionView {
            $crate::auth_require_any!(::icu::auth::is_whitelisted);

            let wallet = msg_caller();

            // Register the session - trap on failure
            if let Err(e) = $crate::state::delegation::DelegationRegistry::register_session(
                wallet,
                args.clone(),
            ) {
                ic_cdk::trap(&format!("Failed to register session: {:?}", e));
            }

            // Return the delegation details directly
            ::icu::state::delegation::DelegationSessionView {
                session_pid: args.session_pid,
                wallet_pid: wallet,
                expires_at: $crate::utils::time::now_secs() + args.duration_secs,
                is_expired: false,
            }
        }

        #[::icu::cdk::update]
        async fn icu_delegation_revoke(pid: ::candid::Principal) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::state::delegation::DelegationRegistry::revoke_session_or_wallet(pid)
        }

        // List all delegation sessions (admin only)
        #[::icu::cdk::query]
        async fn icu_delegation_list_all()
        -> Result<Vec<::icu::state::delegation::DelegationSessionView>, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            Ok($crate::state::delegation::DelegationRegistry::list_all_sessions())
        }

        // List sessions by wallet (admin only)
        #[::icu::cdk::query]
        async fn icu_delegation_list_by_wallet(
            wallet_pid: ::candid::Principal,
        ) -> Result<Vec<::icu::state::delegation::DelegationSessionView>, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            Ok($crate::state::delegation::DelegationRegistry::list_sessions_by_wallet(wallet_pid))
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

        #[::icu::cdk::query]
        fn icu_shard_registry() -> ::icu::memory::CanisterShardRegistryView {
            $crate::memory::CanisterShardRegistry::export()
        }

        #[::icu::cdk::query]
        fn icu_shard_lookup_pool(
            item: ::candid::Principal,
            pool: String,
        ) -> Option<::candid::Principal> {
            let pool = ::icu::memory::PoolName(pool);
            $crate::memory::CanisterShardRegistry::get_item_partition(&item, &pool)
        }

        #[::icu::cdk::update]
        async fn icu_shard_register(
            pid: ::candid::Principal,
            pool: String,
            capacity: u32,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;
            $crate::memory::CanisterShardRegistry::register(
                pid,
                ::icu::memory::PoolName(pool),
                capacity,
            );

            Ok(())
        }

        #[::icu::cdk::update]
        async fn icu_shard_audit() -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;
            $crate::memory::CanisterShardRegistry::audit_and_fix_counts();

            Ok(())
        }

        // Drain items from a shard (admin only). Optionally limited by max_moves.
        #[::icu::cdk::update]
        async fn icu_shard_drain(
            pool: String,
            shard_pid: ::candid::Principal,
            max_moves: u32,
        ) -> Result<u32, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            let hub_type = ::icu::memory::CanisterState::get_type()
                .ok_or_else(|| ::icu::Error::custom("unknown canister type"))?;

            $crate::ops::shard::drain_shard(&hub_type, &pool, shard_pid, max_moves).await
        }

        // Rebalance a pool across existing shards (admin only).
        #[::icu::cdk::update]
        async fn icu_shard_rebalance(pool: String, max_moves: u32) -> Result<u32, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            $crate::ops::shard::rebalance_pool(&pool, max_moves)
        }

        // Decommission an empty shard (admin only).
        #[::icu::cdk::update]
        async fn icu_shard_decommission(
            shard_pid: ::candid::Principal,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            $crate::ops::shard::decommission_shard(shard_pid)
        }
    };
}
