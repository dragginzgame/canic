// icu_endpoints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        // icu_canister_upgrade_children
        // canister_id : None means upgrade all children
        #[::icu::ic::update]
        async fn icu_canister_upgrade_children(
            canister_id: Option<::candid::Principal>,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            // send a request for each matching canister
            for (child_pid, _) in ::icu::memory::ChildIndex::get_data() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    $crate::interface::request::canister_upgrade_request(child_pid).await?
                }
            }

            Ok(())
        }

        // icu_app_state_cascade
        #[::icu::ic::update]
        async fn icu_app_state_cascade(
            data: ::icu::memory::AppStateData,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            // set state and cascade
            $crate::memory::AppState::set_data(data);
            $crate::interface::cascade::app_state_cascade().await?;

            Ok(())
        }

        // icu_subnet_index_cascade
        #[::icu::ic::update]
        async fn icu_subnet_index_cascade(
            data: ::icu::memory::SubnetIndexData,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            // set index and cascade
            $crate::memory::SubnetIndex::set_data(data);
            $crate::interface::cascade::subnet_index_cascade().await?;

            Ok(())
        }

        //
        // IC API ENDPOINTS
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[::icu::ic::query]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::ic::api::msg_cycles_accept(max_amount)
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
        // ICU DELEGATION ENDPOINTS
        //

        #[::icu::ic::update]
        async fn icu_delegation_register(
            args: ::icu::state::RegisterDelegationArgs,
        ) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == wallet_pid
            // or a parent canister
            let expected = args.wallet_pid;
            $crate::auth_require_any!(is_parent, move |caller| { is_principal(caller, expected) })?;

            $crate::state::DelegationList::register_delegation(args)
        }

        #[::icu::ic::update]
        async fn icu_delegation_revoke(pid: Principal) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::state::DelegationList::revoke_delegation(pid)
        }
    };
}
