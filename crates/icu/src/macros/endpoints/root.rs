// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // icu_app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::icu::cdk::update]
        async fn icu_app(cmd: ::icu::memory::app::AppCommand) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::memory::AppState::command(cmd)?;

            let bundle = ::icu::ops::sync::SyncBundle::with_app_state()?;
            ::icu::ops::sync::cascade_children(&bundle).await?;

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
            $crate::auth_require_any!(::icu::auth::is_app)?;

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

        //
        // SUBNET ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icu_subnet_registry() -> Vec<::icu::memory::CanisterEntry> {
            $crate::memory::SubnetRegistry::export()
        }

        //
        // POOL ENDPOINTS
        //

        #[update]
        async fn icu_pool_create_canister() -> Result<Principal, ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::ops::pool::create_pool_canister().await
        }

        #[update]
        async fn icu_pool_move_canister(pid: Principal) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            ::icu::ops::pool::move_canister_to_pool(pid).await
        }

        //
        // MEMORY ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icu_canister_pool() -> ::icu::memory::CanisterPoolView {
            $crate::memory::CanisterPool::export()
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

// icu_endpoints_nonroot
#[macro_export]
macro_rules! icu_endpoints_nonroot {
    () => {
        //
        // ICU SUBNET VIEW
        //

        #[::icu::cdk::query]
        fn icu_subnet_children() -> Vec<::icu::memory::CanisterEntry> {
            $crate::memory::subnet::SubnetChildren::export()
        }

        #[::icu::cdk::query]
        fn icu_subnet_directory() -> Vec<::icu::memory::CanisterEntry> {
            $crate::memory::subnet::SubnetDirectory::export()
        }

        #[::icu::cdk::query]
        fn icu_subnet_parents() -> Vec<::icu::memory::CanisterEntry> {
            $crate::memory::subnet::SubnetParents::export()
        }

        //
        // SYNC ENDPOINTS
        //

        #[::icu::cdk::update]
        async fn icu_sync_cascade(
            bundle: ::icu::ops::sync::SyncBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::sync::save_state(&bundle)?;
            $crate::ops::sync::cascade_children(&bundle).await
        }
    };
}
