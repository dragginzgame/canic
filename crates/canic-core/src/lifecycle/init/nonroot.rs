use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    dto::abi::v1::CanisterInitPayload,
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    log,
    log::Topic,
    ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
    workflow::{self, runtime::timer::TimerWorkflow},
};
use std::time::Duration;

pub fn init_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    payload: CanisterInitPayload,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_compiled_config(config, config_source) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (config_path={config_path}): {err}"),
        );
    }

    if let Err(err) = workflow::runtime::init_nonroot_canister(role, payload) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(LifecyclePhase::Init, err);
    }

    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Completed,
    );
}

pub fn schedule_init_nonroot_bootstrap() {
    LifecycleMetricsApi::record_bootstrap(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Scheduled,
    );
    BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT_SCHEDULED);

    TimerWorkflow::set_application_once(
        Duration::ZERO,
        "canic:bootstrap:init_nonroot_canister",
        async {
            BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::Init,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Started,
            );
            if let Err(err) = workflow::bootstrap::nonroot::bootstrap_init_nonroot_canister().await
            {
                LifecycleMetricsApi::record_bootstrap(
                    LifecycleMetricPhase::Init,
                    LifecycleMetricRole::Nonroot,
                    LifecycleMetricOutcome::Failed,
                );
                BootstrapStatusOps::mark_failed(format!("non-root bootstrap failed (init): {err}"));
                log!(
                    Topic::Init,
                    Error,
                    "non-root bootstrap failed (init): {err}"
                );
                return;
            }
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::Init,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Completed,
            );
        },
    );
}
