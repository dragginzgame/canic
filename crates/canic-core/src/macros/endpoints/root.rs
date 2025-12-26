// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // modify app-level state
        // eventually this will cascade down from an orchestrator canister
        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_app(
            cmd: ::canic::core::ops::state::AppCommand,
        ) -> Result<(), ::canic::Error> {
            ::canic::core::ops::orchestration::AppStateOrchestrator::apply_command(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::ops::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res = $crate::ops::rpc::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(auth_any(::canic::core::auth::is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::core::ops::rpc::Request,
        ) -> Result<::canic::core::ops::rpc::Response, ::canic::Error> {
            let response = $crate::ops::rpc::response(request).await?;

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
            $crate::ops::config::ConfigOps::export_toml()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_app_subnet_registry() -> ::canic::core::ops::topology::AppSubnetRegistryView {
            $crate::ops::topology::AppSubnetRegistryOps::export()
        }

        #[canic_query]
        fn canic_subnet_canister_registry()
        -> ::canic::core::ops::topology::subnet::SubnetCanisterRegistryView {
            $crate::ops::topology::SubnetCanisterRegistryOps::export()
        }

        //
        // CANISTER POOL
        //

        #[canic_query]
        async fn canic_pool_list()
        -> Result<::canic::core::ops::pool::CanisterPoolView, ::canic::Error> {
            Ok($crate::ops::pool::PoolOps::export())
        }

        #[canic_update(auth_any(::canic::core::auth::is_controller))]
        async fn canic_pool_admin(
            cmd: ::canic::core::ops::pool::PoolAdminCommand,
        ) -> Result<::canic::core::ops::pool::PoolAdminResponse, ::canic::Error> {
            ::canic::core::ops::pool::PoolOps::admin(cmd).await
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
            bundle: ::canic::core::ops::orchestration::cascade::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::ops::orchestration::cascade::state::nonroot_cascade_state(&bundle).await
        }

        #[canic_update(auth_any(::canic::core::auth::is_parent))]
        async fn canic_sync_topology(
            bundle: ::canic::core::ops::orchestration::cascade::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::ops::orchestration::cascade::topology::nonroot_cascade_topology(&bundle).await
        }
    };
}
