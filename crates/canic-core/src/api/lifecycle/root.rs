use crate::{config::schema::ConfigModel, dto::subnet::SubnetIdentity, lifecycle};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    pub fn init_root_canister_before_bootstrap(
        identity: SubnetIdentity,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::root::init_root_canister_before_bootstrap(
            identity,
            config,
            config_source,
            config_path,
        );
    }

    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::upgrade::root::post_upgrade_root_canister_before_bootstrap(
            config,
            config_source,
            config_path,
        );
    }
}
