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

        ///
        /// IC API ENDPOINTS
        /// these are specific endpoints defined by the IC spec
        ///

        // ic_cycles_accept
        #[$crate::ic::update]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::ic::api::msg_cycles_accept(max_amount)
        }

        ///
        /// ICU STATE ENDPOINTS
        ///

        // app_state
        #[$crate::ic::query]
        fn app_state() -> ::actor::state::core::AppStateData {
            $crate::state::APP_STATE.with(|this| this.get_data())
        }

        // canister_state
        #[$crate::ic::query]
        fn canister_state() -> ::actor::state::core::CanisterStateData {
            $crate::state::CANISTER_STATE.with(|this| this.get_data())
        }

        // child_index
        #[$crate::ic::query]
        fn child_index() -> ::actor::state::core::ChildIndexData {
            $crate::state::CHILD_INDEX.with(|this| this.get_data())
        }

        // subnet_index
        #[$crate::ic::query]
        fn subnet_index() -> ::actor::state::core::SubnetIndexData {
            $crate::state::SUBNET_INDEX.with(|this| this.get_data())
        }
    };
}
