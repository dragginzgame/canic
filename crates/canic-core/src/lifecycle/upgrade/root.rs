use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    lifecycle::{LifecyclePhase, config_with_current_root_controller, lifecycle_trap},
    ops::runtime::env::EnvOps,
    workflow,
};

pub fn post_upgrade_root_canister_before_bootstrap(
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Started,
    );

    let config = config_with_current_root_controller(config);
    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    let memory_summary = match workflow::runtime::init_memory_registry_post_upgrade() {
        Ok(summary) => summary,
        Err(err) => {
            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Root,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    };

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
    if let Err(err) =
        workflow::runtime::post_upgrade_root_canister_after_memory_init(memory_summary)
    {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(LifecyclePhase::PostUpgrade, err);
    }

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Completed,
    );
}
