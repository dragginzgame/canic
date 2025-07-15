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
            ::icu::auth_require_any!(::icu::auth::is_controller)?;

            // send a request for each matching canister
            for (child_pid, path) in ::icu::interface::memory::canister::child_index::get_data() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    ::icu::interface::request::canister_upgrade_request(child_pid, &path).await?
                }
            }

            Ok(())
        }

        // icu_app_state_cascade
        #[::icu::ic::update]
        async fn icu_app_state_cascade(
            data: ::icu::memory::app::AppStateData,
        ) -> Result<(), ::icu::Error> {
            ::icu::auth_require_any!(::icu::auth::is_parent)?;

            // set state and cascade
            ::icu::interface::memory::app::state::set_data(data)?;
            ::icu::interface::cascade::app_state_cascade().await?;

            Ok(())
        }

        // icu_subnet_index_cascade
        #[::icu::ic::update]
        async fn icu_subnet_index_cascade(
            data: ::icu::memory::subnet::SubnetIndexData,
        ) -> Result<(), ::icu::Error> {
            ::icu::auth_require_any!(::icu::auth::is_parent)?;

            // set index and cascade
            ::icu::interface::memory::subnet::index::set_data(data);
            ::icu::interface::cascade::subnet_index_cascade().await?;

            Ok(())
        }

        //
        // IC API ENDPOINTS
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[::icu::ic::query]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            ::icu::ic::api::msg_cycles_accept(max_amount)
        }

        //
        // ICU HELPERS
        //

        // icu_canister_cycle_balance
        #[::icu::ic::query]
        fn icu_canister_cycle_balance() -> u128 {
            ::icu::ic::api::canister_cycle_balance()
        }

        // icu_canister_version
        #[::icu::ic::query]
        fn icu_canister_version() -> u64 {
            ::icu::ic::api::canister_version()
        }

        // icu_time
        #[::icu::ic::query]
        fn icu_time() -> u64 {
            ::icu::ic::api::time()
        }

        //
        // ICU STATE ENDPOINTS
        //

        // icu_memory_registry
        #[::icu::ic::query]
        fn icu_memory_registry() -> ::icu::memory::registry::RegistryData {
            ::icu::interface::memory::registry::get_data()
        }

        // icu_app_state
        #[::icu::ic::query]
        fn icu_app_state() -> ::icu::memory::app::AppStateData {
            ::icu::interface::memory::app::state::get_data()
        }

        // icu_canister_state
        #[::icu::ic::query]
        fn icu_canister_state() -> ::icu::memory::canister::CanisterStateData {
            ::icu::interface::memory::canister::state::get_data()
        }

        // icu_child_index
        #[::icu::ic::query]
        fn icu_child_index() -> ::icu::memory::canister::ChildIndexData {
            ::icu::interface::memory::canister::child_index::get_data()
        }

        // icu_subnet_index
        #[::icu::ic::query]
        fn icu_subnet_index() -> ::icu::memory::subnet::SubnetIndexData {
            ::icu::interface::memory::subnet::index::get_data()
        }

        // icu_canister_registry
        #[::icu::ic::query]
        fn icu_canister_registry()
        -> Result<::icu::state::root::canister_registry::CanisterRegistryInfo, ::icu::Error> {
            ::icu::interface::state::root::canister_registry::get_info()
        }
    };
}
