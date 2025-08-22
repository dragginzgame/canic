// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // icu_app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::ic::update]
        async fn icu_app(cmd: ::icu::memory::app_state::AppCommand) -> Result<(), ::icu::Error> {
            ::icu::memory::AppState::command(cmd)?;

            let bundle = ::icu::interface::state::StateBundle::app_state();
            ::icu::interface::state::cascade(&bundle).await?;

            Ok(())
        }

        // icu_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[::icu::ic::update]
        async fn icu_response(
            request: ::icu::interface::request::Request,
        ) -> Result<::icu::interface::response::Response, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_root, ::icu::auth::is_app)?;

            let response = ::icu::interface::response::response(request).await?;

            Ok(response)
        }

        // icu_canister_status
        // this can be called via root as root is the master controller
        #[::icu::ic::update]
        async fn icu_canister_status(
            pid: Principal,
        ) -> Result<::icu::ic::mgmt::CanisterStatusResult, ::icu::Error> {
            ::icu::interface::ic::canister_status(pid).await
        }

        ///
        /// POOL ENDPOINTS
        ///

        #[update]
        async fn icu_create_pool_canister() -> Result<Principal, Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::interface::ic::create_pool_canister().await
        }

        #[update]
        async fn icu_move_canister_to_pool(pid: Principal) -> Result<(), Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::interface::ic::move_canister_to_pool(pid).await
        }

        ///
        /// MEMORY ENDPOINTS
        ///

        #[::icu::ic::query]
        fn icu_canister_pool() -> ::icu::memory::CanisterPoolView {
            $crate::memory::CanisterPool::export()
        }

        #[::icu::ic::query]
        fn icu_canister_registry() -> ::icu::memory::CanisterRegistryView {
            $crate::memory::CanisterRegistry::export()
        }
    };
}
