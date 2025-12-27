// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // modify app-level state
        // eventually this will cascade down from an orchestrator canister
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_app(
            cmd: ::canic::core::ops::storage::state::AppCommand,
        ) -> Result<(), ::canic::Error> {
            ::canic::core::workflow::app::AppStateOrchestrator::apply_command(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::ops::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res = $crate::ops::rpc::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(auth_any(::canic::core::access::auth::is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::core::ops::rpc::Request,
        ) -> Result<::canic::core::ops::rpc::Response, ::canic::Error> {
            let response = $crate::workflow::rpc::handler::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[canic_update(auth_any(
            ::canic::core::access::auth::is_root,
            ::canic::core::access::auth::is_controller
        ))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::Error> {
            $crate::ops::ic::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::ops::config::ConfigOps::export_toml()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_APP_REGISTRY() -> ::canic::core::ops::storage::registry::AppRegistryView {
            $crate::ops::storage::registry::AppRegistryOps::export()
        }

        #[canic_query]
        fn canic_SUBNET_REGISTRY()
        -> ::canic::core::ops::storage::registry::subnet::SubnetRegistryView {
            $crate::ops::storage::registry::SubnetRegistryOps::export()
        }

        //
        // CANISTER POOL
        //

        #[canic_query]
        async fn canic_pool_list()
        -> Result<::canic::core::ops::storage::pool::CanisterPoolView, ::canic::Error> {
            Ok($crate::ops::storage::pool::PoolOps::export())
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_pool_admin(
            cmd: ::canic::core::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::core::dto::pool::PoolAdminResponse, ::canic::Error> {
            ::canic::core::workflow::pool::admin::handle_admin(cmd).await
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

        #[canic_update(auth_any(::canic::core::access::auth::is_parent))]
        async fn canic_sync_state(
            bundle: ::canic::core::workflow::cascade::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::workflow::cascade::state::nonroot_cascade_state(&bundle).await
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_parent))]
        async fn canic_sync_topology(
            bundle: ::canic::core::workflow::cascade::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::workflow::cascade::topology::nonroot_cascade_topology(&bundle).await
        }
    };
}
