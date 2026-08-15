use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    ops::runtime::env::EnvOps,
    workflow,
};

pub fn post_upgrade_root_canister_before_bootstrap(
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) -> bool {
    crate::api::timer::TimerApi::initialize_root_runtime_required();
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("config init failed (config_path={config_path}): {err}"),
        );
    }

    match workflow::runtime::init_memory_registry_post_upgrade() {
        Ok(()) => {}
        Err(err) => {
            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Root,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    }

    if let Err(err) = EnvOps::restore_root() {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("env restore failed (root upgrade): {err}"),
        );
    }
    let sealed = crate::ops::storage::authority_restore::AuthorityRestoreFenceOps::is_sealed_for(
        crate::ops::ic::IcOps::canister_self(),
    )
    .unwrap_or_else(|error| {
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("authority restore fence recovery failed: {error}"),
        )
    });
    crate::api::timer::TimerApi::restore_snapshot_suspension(sealed);
    let active = match workflow::runtime::post_upgrade_root_canister_after_memory_init() {
        Ok(active) => active,
        Err(err) => {
            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Root,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    };

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Completed,
    );
    active
}
