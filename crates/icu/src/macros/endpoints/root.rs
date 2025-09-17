// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // icu_app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::cdk::update]
        async fn icu_app(cmd: ::icu::memory::app_state::AppCommand) -> Result<(), ::icu::Error> {
            ::icu::memory::AppState::command(cmd)?;

            let bundle = ::icu::ops::state::StateBundle::app_state();
            ::icu::ops::state::cascade(&bundle).await?;

            Ok(())
        }

        // icu_canister_upgrade
        #[::icu::cdk::update]
        async fn icu_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::icu::ops::response::UpgradeCanisterResponse, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            let res = $crate::ops::request::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // icu_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[::icu::cdk::update]
        async fn icu_response(
            request: ::icu::ops::request::Request,
        ) -> Result<::icu::ops::response::Response, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_root, ::icu::auth::is_app)?;

            let response = $crate::ops::response::response(request).await?;

            Ok(response)
        }

        // icu_canister_status
        // this can be called via root as root is the master controller
        #[::icu::cdk::update]
        async fn icu_canister_status(
            pid: Principal,
        ) -> Result<::icu::cdk::mgmt::CanisterStatusResult, ::icu::Error> {
            $crate::interface::ic::canister_status(pid).await
        }

        ///
        /// POOL ENDPOINTS
        ///

        #[update]
        async fn icu_create_pool_canister() -> Result<Principal, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::ops::pool::create_pool_canister().await
        }

        #[update]
        async fn icu_move_canister_to_pool(pid: Principal) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::ops::pool::move_canister_to_pool(pid).await
        }

        ///
        /// MEMORY ENDPOINTS
        ///

        #[::icu::cdk::query]
        fn icu_canister_pool() -> ::icu::memory::CanisterPoolView {
            $crate::memory::CanisterPool::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_registry() -> ::icu::memory::CanisterRegistryView {
            $crate::memory::CanisterRegistry::export()
        }

        //
        // STATE ENDPOINTS
        //

        #[::icu::cdk::query]
        async fn icu_config() -> Result<String, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            $crate::config::Config::to_toml()
        }
    };
}
