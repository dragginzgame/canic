// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // modify app-level state
        // eventually this will cascade down from an orchestrator canister
        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_app(
            cmd: ::canic::core::ops::storage::state::AppCommand,
        ) -> Result<(), ::canic::Error> {
            ::canic::core::ops::storage::state::AppStateOps::command(cmd)?;

            let bundle = ::canic::core::ops::sync::state::StateBundle::new().with_app_state();
            ::canic::core::ops::sync::state::cascade_root_state(bundle).await?;

            Ok(())
        }

        // canic_canister_upgrade
        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::ops::request::UpgradeCanisterResponse, ::canic::Error> {
            let res = $crate::ops::request::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(auth_any(::canic::core::auth::is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::core::ops::request::Request,
        ) -> Result<::canic::core::ops::request::Response, ::canic::Error> {
            let response = $crate::ops::request::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[canic_update(auth_any(::canic::core::auth::is_root, ::canic::core::auth::is_controller))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::Error> {
            $crate::ops::ic::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(auth_any(::canic::core::auth::is_controller))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::config::Config::to_toml()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_app_subnet_registry()
        -> ::canic::core::ops::storage::topology::AppSubnetRegistryView {
            $crate::ops::storage::topology::AppSubnetRegistryOps::export()
        }

        #[canic_query]
        fn canic_subnet_canister_registry()
        -> ::canic::core::ops::storage::topology::subnet::SubnetCanisterRegistryView {
            $crate::ops::storage::topology::SubnetCanisterRegistryOps::export()
        }

        //
        // CANISTER RESERVE
        //

        #[canic_query]
        async fn canic_reserve_list()
        -> Result<::canic::core::ops::reserve::CanisterReserveView, ::canic::Error> {
            Ok($crate::ops::reserve::CanisterReserveOps::export())
        }

        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_reserve_admin(
            cmd: ::canic::core::ops::reserve::CanisterReserveAdminCommand,
        ) -> Result<::canic::core::ops::reserve::CanisterReserveAdminResponse, ::canic::Error> {
            ::canic::core::ops::reserve::CanisterReserveOps::admin(cmd).await
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

        #[canic_update(auth_any(::canic::core::auth::is_parent))]
        async fn canic_sync_state(
            bundle: ::canic::core::ops::sync::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::ops::sync::state::nonroot_cascade_state(&bundle).await
        }

        #[canic_update(auth_any(::canic::core::auth::is_parent))]
        async fn canic_sync_topology(
            bundle: ::canic::core::ops::sync::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::ops::sync::topology::nonroot_cascade_topology(&bundle).await
        }
    };
}
