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
    workflow::{self, runtime::timer::TimerWorkflow},
};
use std::time::Duration;

pub fn init_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    payload: CanisterInitPayload,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
    with_role_attestation_refresh: bool,
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
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    if let Err(err) =
        workflow::runtime::init_nonroot_canister(role, payload, with_role_attestation_refresh)
    {
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

pub fn schedule_init_nonroot_bootstrap(args: Option<Vec<u8>>) {
    LifecycleMetricsApi::record_bootstrap(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Scheduled,
    );

    TimerWorkflow::set(
        Duration::ZERO,
        "canic:bootstrap:init_nonroot_canister",
        async move {
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::Init,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Started,
            );
            if let Err(err) =
                workflow::bootstrap::nonroot::bootstrap_init_nonroot_canister(args).await
            {
                LifecycleMetricsApi::record_bootstrap(
                    LifecycleMetricPhase::Init,
                    LifecycleMetricRole::Nonroot,
                    LifecycleMetricOutcome::Failed,
                );
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
