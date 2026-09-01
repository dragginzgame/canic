use crate::{
    config::{RoleRuntimeAuthority, schema::ConfigModel},
    dto::fleet_subnet_root::FleetSubnetRootInitArgs,
    lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    pub fn init_root_canister_before_bootstrap(
        args: FleetSubnetRootInitArgs,
        embedded_release_build_id: Option<&str>,
        runtime_authority: RoleRuntimeAuthority,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::root::init_root_canister_before_bootstrap(
            args,
            embedded_release_build_id,
            runtime_authority,
            config,
            config_source,
            config_path,
        );
    }

    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        embedded_release_build_id: Option<&str>,
        runtime_authority: RoleRuntimeAuthority,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::root::post_upgrade_root_canister_before_bootstrap(
            embedded_release_build_id,
            runtime_authority,
            config,
            config_source,
            config_path,
        )
    }
}
