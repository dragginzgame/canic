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

            ::icu::memory::app::AppState::command(cmd)?;

            let bundle = ::icu::ops::sync::state::StateBundle::root();
            ::icu::ops::sync::state::root_cascade(bundle).await?;

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
        // (on root, these views are returned by the registry)
        //

        #[::icu::cdk::query]
        fn icu_subnet_registry() -> Vec<::icu::memory::CanisterEntry> {
            $crate::memory::subnet::SubnetRegistry::all()
        }

        #[::icu::cdk::query]
        fn icu_subnet_children() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetRegistry::children(::icu::cdk::api::canister_self())
        }

        #[::icu::cdk::query]
        fn icu_subnet_directory() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetRegistry::directory()
        }

        #[::icu::cdk::query]
        fn icu_subnet_parents() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetRegistry::parents(::icu::cdk::api::canister_self())
        }

        //
        // POOL ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icu_pool_list() -> ::icu::memory::root::CanisterPoolView {
            $crate::memory::root::CanisterPool::export()
        }

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
        fn icu_subnet_children() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetChildren::export()
        }

        #[::icu::cdk::query]
        fn icu_subnet_directory() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetDirectory::export()
        }

        #[::icu::cdk::query]
        fn icu_subnet_parents() -> Vec<::icu::memory::CanisterView> {
            $crate::memory::subnet::SubnetParents::export()
        }

        //
        // SYNC ENDPOINTS
        //

        #[::icu::cdk::update]
        async fn icu_sync_state(
            bundle: ::icu::ops::sync::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::sync::state::save_state(&bundle)?;
            $crate::ops::sync::state::cascade_children(&bundle).await
        }

        #[::icu::cdk::update]
        async fn icu_sync_topology(
            bundle: ::icu::ops::sync::topology::TopologyBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::sync::topology::save_state(&bundle)?;
            $crate::ops::sync::topology::cascade_children(&bundle).await
        }
    };
}
