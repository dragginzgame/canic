// icu_endpoints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        //
        // IC API ENDPOINTS (IMPORTANT!!)
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[::icu::ic::query]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::ic::api::msg_cycles_accept(max_amount)
        }

        // icu_canister_upgrade_children
        // canister_id : None means upgrade all children
        #[::icu::ic::update]
        async fn icu_canister_upgrade_children(
            canister_id: Option<::candid::Principal>,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            // send a request for each matching canister
            for (child_pid, _) in $crate::memory::ChildIndex::export() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    $crate::interface::request::upgrade_canister_request(child_pid).await?
                }
            }

            Ok(())
        }

        #[::icu::ic::update]
        async fn icu_state_update(
            bundle: ::icu::interface::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::interface::state::save_state(&bundle);

            Ok(())
        }

        #[::icu::ic::update]
        async fn icu_state_cascade(
            bundle: ::icu::interface::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::interface::state::save_state(&bundle);
            $crate::interface::state::cascade(&bundle).await
        }

        //
        // ICRC ENDPOINTS
        //

        #[::icu::ic::query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::state::Icrc10Registry::supported_standards()
        }

        #[::icu::ic::query]
        async fn icrc21_canister_call_consent_message(
            req: ::icu::interface::icrc::Icrc21ConsentMessageRequest,
        ) -> ::icu::interface::icrc::Icrc21ConsentMessageResponse {
            $crate::state::Icrc21Registry::consent_message(req)
        }

        //
        // ICU CANISTER HELPERS
        //

        #[::icu::ic::query]
        fn icu_canister_cycle_balance() -> u128 {
            $crate::ic::api::canister_cycle_balance()
        }

        #[::icu::ic::query]
        fn icu_canister_version() -> u64 {
            $crate::ic::api::canister_version()
        }

        #[::icu::ic::query]
        fn icu_time() -> u64 {
            $crate::ic::api::time()
        }

        //
        // ICU MEMORY HELPERS
        //

        #[::icu::ic::query]
        fn icu_app_state() -> ::icu::memory::AppStateData {
            $crate::memory::AppState::export()
        }

        #[::icu::ic::query]
        fn icu_canister_state() -> ::icu::memory::CanisterStateData {
            $crate::memory::CanisterState::export()
        }

        #[::icu::ic::query]
        fn icu_child_index() -> ::icu::memory::ChildIndexData {
            $crate::memory::ChildIndex::export()
        }

        #[::icu::ic::query]
        fn icu_subnet_index() -> ::icu::memory::SubnetIndexData {
            $crate::memory::SubnetIndex::export()
        }

        //
        // ICU CONFIG ENDPOINTS
        //

        #[::icu::ic::query]
        fn icu_config() -> Result<::icu::config::ConfigData, ::icu::Error> {
            $crate::config::Config::get()
        }

        //
        // ICU DELEGATION ENDPOINTS
        //

        #[::icu::ic::query]
        async fn icu_delegation_get(
            session_pid: Principal,
        ) -> Result<::icu::state::DelegationSessionView, ::icu::Error> {
            $crate::state::DelegationRegistry::get(session_pid)
        }

        #[::icu::ic::update]
        async fn icu_delegation_track(
            session_pid: Principal,
        ) -> Result<::icu::state::DelegationSessionView, ::icu::Error> {
            $crate::state::DelegationRegistry::track(msg_caller(), session_pid)
        }

        #[::icu::ic::update]
        async fn icu_delegation_register(
            args: ::icu::state::RegisterSessionArgs,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_whitelisted)?;

            $crate::state::DelegationRegistry::register_session(msg_caller(), args)
        }

        #[::icu::ic::update]
        async fn icu_delegation_revoke(pid: Principal) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::state::DelegationRegistry::revoke_session_or_wallet(pid)
        }
    };
}
