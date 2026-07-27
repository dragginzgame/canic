use crate::{
    config::schema::ConfigModel, dto::fleet_subnet_root::FleetSubnetRootInitArgs, lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    pub fn init_root_canister_before_bootstrap(
        args: FleetSubnetRootInitArgs,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::root::init_root_canister_before_bootstrap(
            args,
            config,
            config_source,
            config_path,
        );
    }

    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::root::post_upgrade_root_canister_before_bootstrap(
            config,
            config_source,
            config_path,
        )
    }
}
