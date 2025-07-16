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
            for (child_pid, path) in ::icu::memory::ChildIndex::get_data() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    $crate::interface::request::canister_upgrade_request(child_pid, &path).await?
                }
            }

            Ok(())
        }

        // icu_app_state_cascade
        #[::icu::ic::update]
        async fn icu_app_state_cascade(
            data: ::icu::memory::AppStateData,
        ) -> Result<(), ::icu::Error> {
            ::icu::auth_require_any!(::icu::auth::is_parent)?;

            // set state and cascade
            ::icu::memory::AppState::set_data(data).map_err(::icu::memory::MemoryError::from)?;
            ::icu::interface::cascade::app_state_cascade().await?;

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
        // ICU HELPERS
        //

        // icu_canister_cycle_balance
        #[::icu::ic::query]
        fn icu_canister_cycle_balance() -> u128 {
            $crate::ic::api::canister_cycle_balance()
        }

        // icu_canister_version
        #[::icu::ic::query]
        fn icu_canister_version() -> u64 {
            $crate::ic::api::canister_version()
        }

        // icu_time
        #[::icu::ic::query]
        fn icu_time() -> u64 {
            $crate::ic::api::time()
        }

        //
        // ICU STATE ENDPOINTS
        //

        // icu_memory_registry
        #[::icu::ic::query]
        fn icu_memory_registry() -> ::icu::memory::RegistryData {
            $crate::memory::memory_registry_data()
        }

        // icu_app_state
        #[::icu::ic::query]
        fn icu_app_state() -> ::icu::memory::AppStateData {
            $crate::memory::AppState::get_data()
        }

        // icu_canister_state
        #[::icu::ic::query]
        fn icu_canister_state() -> ::icu::memory::CanisterStateData {
            $crate::memory::CanisterState::get_data()
        }

        // icu_child_index
        #[::icu::ic::query]
        fn icu_child_index() -> ::icu::memory::ChildIndexData {
            $crate::memory::ChildIndex::get_data()
        }

        // icu_subnet_index
        #[::icu::ic::query]
        fn icu_subnet_index() -> ::icu::memory::SubnetIndexData {
            $crate::memory::SubnetIndex::get_data()
        }

        // icu_canister_registry
        #[::icu::ic::query]
        fn icu_canister_registry() -> ::icu::state::CanisterRegistryData {
            $crate::state::CanisterRegistry::get_data()
        }
    };
}
