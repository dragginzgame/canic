// Generate the endpoint surface for the root orchestrator canister.
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

            let bundle = ::canic::ops::sync::state::StateBundle::new().with_app_state();
            ::canic::ops::sync::state::root_cascade_state(bundle).await?;

            Ok(())
        }

        // canic_canister_upgrade
        #[::canic::cdk::update]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::ops::request::UpgradeCanisterResponse, ::canic::Error> {
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
        ) -> Result<::canic::ops::request::Response, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_registered_to_subnet)?;

            let response = $crate::ops::request::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[::canic::cdk::update]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::cdk::mgmt::CanisterStatusResult, ::canic::Error> {
            $crate::interface::ic::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[::canic::cdk::query]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            $crate::config::Config::to_toml()
        }

        //
        // REGISTRIES + TOPOLOGY
        // on root, the SubnetCanisterRegistry is the main source of truth
        //

        #[::canic::cdk::query]
        fn canic_app_subnet_registry() -> ::canic::memory::topology::AppSubnetRegistryView {
            $crate::memory::topology::AppSubnetRegistry::export()
        }

        #[::canic::cdk::query]
        fn canic_app_canister_registry() -> ::canic::memory::topology::AppSubnetRegistryView {
            $crate::memory::topology::AppSubnetRegistry::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_canister_registry() -> Vec<::canic::memory::CanisterEntry> {
            $crate::memory::topology::SubnetCanisterRegistry::export()
        }

        // children is auto-generated from the registry
        #[::canic::cdk::query]
        fn canic_subnet_canister_children(
            offset: u64,
            limit: u64,
        ) -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetCanisterRegistry::children(
                ::canic::cdk::api::canister_self(),
            )
        }

        //
        // CANISTER RESERVE
        //

        #[::canic::cdk::query]
        async fn canic_reserve_list()
        -> Result<::canic::memory::root::reserve::CanisterReserveView, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            Ok($crate::memory::root::reserve::CanisterReserve::export())
        }

        #[update]
        async fn canic_reserve_create_canister()
        -> Result<::canic::cdk::candid::Principal, ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            ::canic::ops::root::reserve::reserve_create_canister().await
        }

        #[update]
        async fn canic_reserve_import_canister(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_controller)?;

            ::canic::ops::root::reserve::reserve_import_canister(pid).await
        }
    };
}

// Generate the endpoint surface for non-root canisters.
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        //
        // TOPOLOGY (NON AUTHORITATIVE)
        //

        #[::canic::cdk::query]
        fn canic_subnet_canister_children(
            offset: u64,
            limit: u64,
        ) -> Vec<::canic::memory::CanisterSummary> {
            $crate::memory::topology::SubnetCanisterChildren::export()
        }

        //
        // DIRECTORY VIEWS
        //

        #[::canic::cdk::query]
        fn canic_app_directory() -> ::canic::memory::directory::DirectoryView {
            $crate::memory::directory::AppDirectory::export()
        }

        #[::canic::cdk::query]
        fn canic_subnet_directory() -> ::canic::memory::directory::DirectoryView {
            $crate::memory::directory::SubnetDirectory::export()
        }

        //
        // SYNC
        //

        #[::canic::cdk::update]
        async fn canic_sync_state(
            bundle: ::canic::ops::sync::state::StateBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_parent)?;

            $crate::ops::sync::state::nonroot_cascade_state(&bundle).await
        }

        #[::canic::cdk::update]
        async fn canic_sync_topology(
            bundle: ::canic::ops::sync::topology::TopologyBundle,
        ) -> Result<(), ::canic::Error> {
            $crate::auth_require_any!(::canic::auth::is_parent)?;

            $crate::ops::sync::topology::nonroot_cascade_topology(&bundle).await
        }
    };
}
