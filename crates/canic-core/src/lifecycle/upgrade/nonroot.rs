use crate::{
    bootstrap,
    config::schema::ConfigModel,
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    log,
    log::Topic,
    ops::runtime::{env::EnvOps, timer::TimerOps},
    workflow,
};
use std::time::Duration;

pub fn post_upgrade_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
    with_role_attestation_refresh: bool,
) {
    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    let memory_summary = match workflow::runtime::init_memory_registry_post_upgrade() {
        Ok(summary) => summary,
        Err(err) => lifecycle_trap(LifecyclePhase::PostUpgrade, err),
    };

    if let Err(err) = EnvOps::restore_role(role.clone()) {
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("env restore failed (nonroot upgrade): {err}"),
        );
    }
    workflow::runtime::post_upgrade_nonroot_canister_after_memory_init(
        role,
        memory_summary,
        with_role_attestation_refresh,
    );
}

pub fn schedule_post_upgrade_nonroot_bootstrap() {
    TimerOps::set(
        Duration::ZERO,
        "canic:bootstrap:post_upgrade_nonroot_canister",
        async {
            if let Err(err) =
                workflow::bootstrap::nonroot::bootstrap_post_upgrade_nonroot_canister().await
            {
                log!(
                    Topic::Init,
                    Error,
                    "non-root bootstrap failed (post-upgrade): {err}"
                );
            }
        },
    );
}
