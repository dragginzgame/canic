// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::ic::update]
        async fn app(cmd: ::icu::state::core::app_state::AppCommand) -> Result<(), ::icu::Error> {
            ::icu::interface::state::core::app_state::command(cmd)?;
            ::icu::interface::cascade::app_state_cascade().await?;

            Ok(())
        }

        // response
        #[::icu::ic::update]
        async fn response(
            request: ::icu::interface::request::Request,
        ) -> Result<::icu::interface::response::Response, ::icu::Error> {
            let response = ::icu::interface::response::response(request).await?;

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
        #[::icu::ic::update(guard = "::icu::guard::guard_update")]
        async fn icu_canister_upgrade_children(
            canister_id: Option<::candid::Principal>,
        ) -> Result<(), ::icu::Error> {
            //           allow_any(vec![Auth::Controller]).await?;

            // send a request for each matching canister
            for (child_pid, path) in ::icu::interface::state::core::child_index::get_data() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    let req =
                        ::icu::interface::request::Request::new_canister_upgrade(child_pid, &path);

                    if let Err(e) = ::icu::interface::request::request_api(req).await {
                        log!(Log::Warn, "{child_pid} ({path}): {e}");
                    }
                }
            }

            Ok(())
        }

        // icu_app_state_cascade
        #[::icu::ic::update]
        async fn icu_app_state_cascade(
            data: ::icu::state::core::AppStateData,
        ) -> Result<(), ::icu::Error> {
            //    allow_any(vec![Auth::Parent]).await?;

            // set state and cascade
            ::icu::interface::state::core::app_state::set_data(data)?;
            ::icu::interface::cascade::app_state_cascade_api().await?;

            Ok(())
        }

        // icu_subnet_index_cascade
        #[::icu::ic::update]
        async fn icu_subnet_index_cascade(
            data: ::icu::state::core::SubnetIndexData,
        ) -> Result<(), ::icu::Error> {
            //       allow_any(vec![Auth::Parent]).await?;

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
