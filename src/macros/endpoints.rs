// icu_endpo9ints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        // canister_upgrade_children
        // canister_id : None means upgrade all children
        /*
                #[$crate::ic::update(guard = "guard_update")]
                async fn canister_upgrade_children(
                    canister_id: Option<Principal>,
                ) -> Result<(), ActorError> {
                    allow_any(vec![Auth::Controller]).await?;

                    // send a request for each matching canister
                    for (child_id, path) in child_index() {
                        if canister_id.is_none() || canister_id == Some(child_id) {
                            let req = ::actor::interface::request::Request::new_canister_upgrade(
                                child_id,
                                path.clone(),
                            );

                            if let Err(e) = ::actor::interface::request::request_api(req).await {
                                log!(Log::Warn, "{child_id} ({path}): {e}");
                            }
                        }
                    }

                    Ok(())
                }

                // app_state_cascade
                #[$crate::ic::update]
                async fn app_state_cascade(data: ::actor::state::core::AppStateData) -> Result<(), String> {
                    allow_any(vec![Auth::Parent]).await?;

                    // set state and cascade
                    ::actor::interface::state::core::app_state::set_data_api(data)?;
                    ::actor::interface::cascade::app_state_cascade_api().await?;

                    Ok(())
                }

                // subnet_index_cascade
                #[$crate::ic::update]
                async fn subnet_index_cascade(
                    data: ::actor::state::core::SubnetIndexData,
                ) -> Result<(), String> {
                    allow_any(vec![Auth::Parent]).await?;

                    // set index and cascade
                    ::actor::interface::state::core::subnet_index::set_data(data);
                    ::actor::interface::cascade::subnet_index_cascade_api().await?;

                    Ok(())
                }
        */

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

        // icu_app_state
        #[::icu::ic::query]
        fn icu_app_state() -> ::icu::state::core::AppStateData {
            ::icu::state::core::APP_STATE.with_borrow(|this| this.get_data())
        }

        // icu_canister_state
        #[::icu::ic::query]
        fn icu_canister_state() -> ::icu::state::core::CanisterStateData {
            ::icu::state::core::CANISTER_STATE.with_borrow(|this| this.get_data())
        }

        // icu_child_index
        #[::icu::ic::query]
        fn icu_child_index() -> ::icu::state::core::ChildIndexData {
            ::icu::state::core::CHILD_INDEX.with_borrow(|this| this.get_data())
        }

        // icu_subnet_index
        #[::icu::ic::query]
        fn icu_subnet_index() -> ::icu::state::core::SubnetIndexData {
            ::icu::state::core::SUBNET_INDEX.with_borrow(|this| this.get_data())
        }
    };
}
