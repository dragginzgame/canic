// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::mimic::ic::update]
        async fn app(cmd: ::actor::state::core::app_state::AppCommand) -> Result<(), ActorError> {
            ::actor::interface::state::core::app_state::command_api(cmd)?;
            ::actor::interface::cascade::app_state_cascade_api().await?;

            Ok(())
        }

        // response
        #[::mimic::ic::update]
        async fn response(
            request: ::actor::interface::request::Request,
        ) -> Result<::actor::interface::response::Response, ActorError> {
            let response = ::actor::interface::response::response(request).await?;

            Ok(response)
        }
    };
}

// icu_endpoints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        // icu_canister_upgrade_children
        // canister_id : None means upgrade all children
        #[::icu::ic::update(guard = "guard_update")]
        async fn icu_canister_upgrade_children(
            canister_id: Option<Principal>,
        ) -> Result<(), ActorError> {
            allow_any(vec![Auth::Controller]).await?;

            // send a request for each matching canister
            for (child_id, path) in child_index() {
                if canister_id.is_none() || canister_id == Some(child_id) {
                    let req = ::icu::interface::request::Request::new_canister_upgrade(
                        child_id,
                        path.clone(),
                    );

                    if let Err(e) = ::icu::interface::request::request_api(req).await {
                        log!(Log::Warn, "{child_id} ({path}): {e}");
                    }
                }
            }

            Ok(())
        }

        // icu_app_state_cascade
        #[::icu::ic::update]
        async fn icu_app_state_cascade(
            data: ::actor::state::core::AppStateData,
        ) -> Result<(), String> {
            allow_any(vec![Auth::Parent]).await?;

            // set state and cascade
            ::icu::interface::state::core::app_state::set_data_api(data)?;
            ::icu::interface::cascade::app_state_cascade_api().await?;

            Ok(())
        }

        // icu_subnet_index_cascade
        #[::icu::ic::update]
        async fn icu_subnet_index_cascade(
            data: ::actor::state::core::SubnetIndexData,
        ) -> Result<(), String> {
            allow_any(vec![Auth::Parent]).await?;

            // set index and cascade
            ::icu::interface::state::core::subnet_index::set_data(data);
            ::icu::interface::cascade::subnet_index_cascade_api().await?;

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

        // icu_app_state
        #[::icu::ic::query]
        fn icu_app_state() -> ::icu::state::core::AppStateData {
            ::icu::interface::state::core::app_state::get_data()
        }

        // icu_canister_state
        #[::icu::ic::query]
        fn icu_canister_state() -> ::icu::state::core::CanisterStateData {
            ::icu::interface::state::core::canister_state::get_data()
        }

        // icu_child_index
        #[::icu::ic::query]
        fn icu_child_index() -> ::icu::state::core::ChildIndexData {
            ::icu::interface::state::core::child_index::get_data()
        }

        // icu_subnet_index
        #[::icu::ic::query]
        fn icu_subnet_index() -> ::icu::state::core::SubnetIndexData {
            ::icu::interface::state::core::subnet_index::get_data()
        }

        // icu_wasm_info
        #[::icu::ic::query]
        fn icu_wasm_info() -> Result<::icu::state::wasm::WasmInfoData, ::icu::Error> {
            ::icu::interface::state::wasm::info()
        }
    };
}
