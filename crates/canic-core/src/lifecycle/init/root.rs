use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::{RoleRuntimeAuthority, schema::ConfigModel},
    dto::fleet_subnet_root::FleetSubnetRootInitArgs,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    workflow,
};

pub fn init_root_canister_before_bootstrap(
    args: FleetSubnetRootInitArgs,
    embedded_release_build_id: Option<&str>,
    runtime_authority: RoleRuntimeAuthority,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    crate::api::timer::TimerApi::initialize_root_runtime_required();
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Root,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (config_path={config_path}): {err}"),
        );
    }
    if let Err(err) =
        bootstrap::init_role_runtime_authority(&crate::ids::CanisterRole::ROOT, runtime_authority)
    {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("runtime authority init failed: {err}"),
        );
    }

    if let Err(err) = workflow::runtime::init_root_canister(args, embedded_release_build_id) {
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
