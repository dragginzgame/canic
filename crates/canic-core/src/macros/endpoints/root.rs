// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // root-only app-level state mutation endpoint
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_app(
            cmd: ::canic::core::dto::state::AppCommand,
        ) -> Result<(), ::canic::PublicError> {
            $crate::api::endpoints::canic_app(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::dto::rpc::UpgradeCanisterResponse, ::canic::PublicError> {
            let res = $crate::api::endpoints::canic_canister_upgrade(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(auth_any(::canic::core::access::auth::is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::core::dto::rpc::Request,
        ) -> Result<::canic::core::dto::rpc::Response, ::canic::PublicError> {
            let response = $crate::api::endpoints::canic_response(request).await?;

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
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::PublicError> {
            $crate::api::endpoints::canic_canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_config() -> Result<String, ::canic::PublicError> {
            $crate::api::endpoints::canic_config()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_app_registry() -> ::canic::core::dto::registry::AppRegistryView {
            $crate::api::endpoints::canic_app_registry()
        }

        #[canic_query]
        fn canic_subnet_registry() -> ::canic::core::dto::registry::SubnetRegistryView {
            $crate::api::endpoints::canic_subnet_registry()
        }

        //
        // CANISTER POOL
        //

        #[canic_query]
        async fn canic_pool_list()
        -> Result<::canic::core::dto::pool::CanisterPoolView, ::canic::PublicError> {
            $crate::api::endpoints::canic_pool_list()
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_pool_admin(
            cmd: ::canic::core::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::core::dto::pool::PoolAdminResponse, ::canic::PublicError> {
            $crate::api::endpoints::canic_pool_admin(cmd).await
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
            snapshot: ::canic::core::dto::cascade::StateSnapshotView,
        ) -> Result<(), ::canic::PublicError> {
            $crate::api::endpoints::canic_sync_state(snapshot).await
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_parent))]
        async fn canic_sync_topology(
            snapshot: ::canic::core::dto::cascade::TopologySnapshotView,
        ) -> Result<(), ::canic::PublicError> {
            $crate::api::endpoints::canic_sync_topology(snapshot).await
        }
    };
}
