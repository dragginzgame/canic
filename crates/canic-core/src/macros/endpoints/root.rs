// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // modify app-level state
        // eventually this will cascade down from an orchestrator canister
        #[::canic::cdk::update]
        async fn canic_app(
            cmd: ::canic::core::ops::model::memory::state::AppCommand,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_controller)?;

            ::canic::core::ops::model::memory::state::AppStateOps::command(cmd)?;

            let bundle = ::canic::core::ops::sync::state::StateBundle::new().with_app_state();
            ::canic::core::ops::sync::state::root_cascade_state(bundle).await?;

            Ok(())
        }

        // canic_canister_upgrade
        #[::canic::cdk::update]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::ops::request::UpgradeCanisterResponse, ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_controller)?;

            let res = $crate::ops::request::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[::canic::cdk::update]
        async fn canic_response(
            request: ::canic::core::ops::request::Request,
        ) -> Result<::canic::core::ops::request::Response, ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_registered_to_subnet)?;

            let response = $crate::ops::request::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[::canic::cdk::update]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::Error> {
            auth_require_any!(
                ::canic::core::auth::is_root,
                ::canic::core::auth::is_controller
            )?;

            $crate::interface::ic::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[::canic::cdk::query]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_controller)?;

            $crate::config::Config::to_toml()
        }

        //
        // REGISTRIES
        //

        #[::canic::cdk::query]
        fn canic_app_subnet_registry()
        -> ::canic::core::ops::model::memory::topology::AppSubnetRegistryView {
            $crate::ops::model::memory::topology::AppSubnetRegistryOps::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_canister_registry()
        -> ::canic::core::ops::model::memory::topology::subnet::SubnetCanisterRegistryView {
            $crate::ops::model::memory::topology::SubnetCanisterRegistryOps::export()
        }

        //
        // CANISTER RESERVE
        //

        #[::canic::cdk::query]
        async fn canic_reserve_list()
        -> Result<::canic::core::ops::model::memory::reserve::CanisterReserveView, ::canic::Error> {
            Ok($crate::ops::model::memory::reserve::CanisterReserveOps::export())
        }

        #[update]
        async fn canic_reserve_create_canister()
        -> Result<::canic::cdk::candid::Principal, ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_controller)?;

            ::canic::core::ops::model::memory::reserve::reserve_create_canister().await
        }

        #[update]
        async fn canic_reserve_import_canister(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_controller)?;

            ::canic::core::ops::model::memory::reserve::reserve_import_canister(pid).await
        }
    };
}

// Generate the endpoint surface for non-root canisters.
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        //
        // SYNC
        //

        #[::canic::cdk::update]
        async fn canic_sync_state(
            bundle: ::canic::core::ops::sync::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_parent)?;

            $crate::ops::sync::state::nonroot_cascade_state(&bundle).await
        }

        #[::canic::cdk::update]
        async fn canic_sync_topology(
            bundle: ::canic::core::ops::sync::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::core::auth::is_parent)?;

            $crate::ops::sync::topology::nonroot_cascade_topology(&bundle).await
        }
    };
}
