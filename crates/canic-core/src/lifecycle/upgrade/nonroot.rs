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
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    let memory_summary = match workflow::runtime::init_memory_registry_post_upgrade() {
        Ok(summary) => summary,
        Err(err) => {
            LifecycleMetricsApi::record_runtime(
                LifecycleMetricPhase::PostUpgrade,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Failed,
            );
            lifecycle_trap(LifecyclePhase::PostUpgrade, err);
        }
    };

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
    if let Err(err) = workflow::runtime::post_upgrade_nonroot_canister_after_memory_init(
        role,
        memory_summary,
        with_role_attestation_refresh,
    ) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(LifecyclePhase::PostUpgrade, err);
    }

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Completed,
    );
}

pub fn schedule_post_upgrade_nonroot_bootstrap() {
    LifecycleMetricsApi::record_bootstrap(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Scheduled,
    );

    TimerOps::set(
        Duration::ZERO,
        "canic:bootstrap:post_upgrade_nonroot_canister",
        async {
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
