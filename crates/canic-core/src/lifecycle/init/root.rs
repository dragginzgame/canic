use crate::{
    bootstrap,
    config::schema::ConfigModel,
    dto::subnet::SubnetIdentity,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    workflow,
};

pub fn init_root_canister_before_bootstrap(
    identity: SubnetIdentity,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    if let Err(err) = workflow::runtime::init_root_canister(identity) {
        lifecycle_trap(LifecyclePhase::Init, err);
    }
}
