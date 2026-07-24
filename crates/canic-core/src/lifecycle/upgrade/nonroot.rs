use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    log,
    log::Topic,
    ops::runtime::{
        bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
        env::EnvOps,
    },
    workflow::{self, runtime::timer::TimerWorkflow},
};
use std::time::Duration;

pub fn post_upgrade_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(
        role,
        config,
        config_source,
        config_path,
        workflow::runtime::post_upgrade_nonroot_canister_after_memory_init,
    )
}

pub fn post_upgrade_local_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(
        role,
        config,
        config_source,
        config_path,
        workflow::runtime::post_upgrade_local_nonroot_canister_after_memory_init,
    )
}

fn post_upgrade_nonroot_before_bootstrap(
    role: CanisterRole,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
    restore: fn(CanisterRole) -> Result<bool, crate::InternalError>,
) -> bool {
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Nonroot,
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
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    }

    if let Err(err) = EnvOps::restore_role(role.clone()) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("env restore failed (nonroot upgrade): {err}"),
        );
    }
    let active = match restore(role) {
        Ok(active) => active,
        Err(err) => {
            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    };

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Completed,
    );
    active
}

pub fn schedule_post_upgrade_nonroot_bootstrap() {
    LifecycleMetricsApi::record_bootstrap(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Scheduled,
    );
    BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_UPGRADE_SCHEDULED);

    TimerWorkflow::set_application_once(
        Duration::ZERO,
        "canic:bootstrap:post_upgrade_nonroot_canister",
        async {
            BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_UPGRADE);
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Started,
            );
            if let Err(err) =
                workflow::bootstrap::nonroot::bootstrap_post_upgrade_nonroot_canister().await
            {
                LifecycleMetricsApi::record_bootstrap(
                    LifecycleMetricPhase::PostUpgrade,
                    LifecycleMetricRole::Nonroot,
                    LifecycleMetricOutcome::Failed,
                );
                BootstrapStatusOps::mark_failed(format!(
                    "non-root bootstrap failed (post-upgrade): {err}"
                ));
                log!(
                    Topic::Init,
                    Error,
                    "non-root bootstrap failed (post-upgrade): {err}"
                );
                return;
            }
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Completed,
            );
        },
    );
}
