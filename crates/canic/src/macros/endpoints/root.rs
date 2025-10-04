// canic_endpoints_root
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::canic::cdk::update]
        async fn canic_app(cmd: ::canic::memory::state::AppCommand) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            ::canic::memory::state::AppState::command(cmd)?;

            let bundle = ::canic::ops::sync::state::StateBundle::root();
            ::canic::ops::sync::state::root_cascade(bundle).await?;

            Ok(())
        }

        // canic_canister_upgrade
        #[::canic::cdk::update]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::ops::response::UpgradeCanisterResponse, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            let res = $crate::ops::request::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[::canic::cdk::update]
        async fn canic_response(
            request: ::canic::ops::request::Request,
        ) -> Result<::canic::ops::response::Response, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_app)?;

            let response = $crate::ops::response::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[::canic::cdk::update]
        async fn canic_canister_status(
            pid: Principal,
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::Error> {
            $crate::interface::ic::canister_status(pid).await
        }

        //
        // TOPOLOGY
        // (on root, these views are returned by the registry)
        //

        #[::canic::cdk::query]
        fn canic_subnet_registry() -> Vec<::canic::memory::CanisterEntry> {
            $crate::memory::topology::SubnetTopology::all()
        }

        #[::canic::cdk::query]
        fn canic_subnet_children() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetTopology::children(::canic::cdk::api::canister_self())
        }

        #[::canic::cdk::query]
        fn canic_subnet_directory() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetTopology::directory()
        }

        #[::canic::cdk::query]
        fn canic_subnet_parents() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetTopology::parents(::canic::cdk::api::canister_self())
        }

        //
        // RESERVE
        //

        #[::canic::cdk::query]
        fn canic_reserve_list() -> ::canic::memory::root::CanisterReserveView {
            $crate::memory::root::CanisterReserve::export()
        }

        #[update]
        async fn canic_reserve_create_canister() -> Result<Principal, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            ::canic::ops::reserve::create_reserve_canister().await
        }

        #[update]
        async fn canic_reserve_move_canister(pid: Principal) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            ::canic::ops::reserve::move_canister_to_reserve(pid).await
        }

        //
        // MEMORY CONTEXT
        //

        #[::canic::cdk::query]
        fn canic_subnet_context() -> ::canic::memory::context::SubnetContextData {
            $crate::memory::context::SubnetContext::export()
        }

        //
        // CONFIG
        //

        #[::canic::cdk::query]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            $crate::config::Config::to_toml()
        }
    };
}

// canic_endpoints_nonroot
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        //
        // TOPOLOGY (NOT AUTHORITATIVE)
        //

        #[::canic::cdk::query]
        fn canic_subnet_children() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetChildren::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_directory() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetDirectory::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_parents() -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetParents::export()
        }

        //
        // SYNC
        //

        #[::canic::cdk::update]
        async fn canic_sync_state(
            bundle: ::canic::ops::sync::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_parent)?;

            $crate::ops::sync::state::save_state(&bundle)?;
            $crate::ops::sync::state::cascade_children(&bundle).await
        }

        #[::canic::cdk::update]
        async fn canic_sync_topology(
            bundle: ::canic::ops::sync::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_parent)?;

            $crate::ops::sync::topology::save_state(&bundle)?;
            $crate::ops::sync::topology::cascade_children(&bundle).await
        }
    };
}
