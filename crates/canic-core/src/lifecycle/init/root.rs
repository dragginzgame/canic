use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    dto::subnet::SubnetIdentity,
    lifecycle::{LifecyclePhase, config_with_current_root_controller, lifecycle_trap},
    workflow,
};

pub fn init_root_canister_before_bootstrap(
    identity: SubnetIdentity,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Started,
    );

    let config = config_with_current_root_controller(config);
    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    if let Err(err) = workflow::runtime::init_root_canister(identity) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(LifecyclePhase::Init, err);
    }

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Completed,
    );
}
