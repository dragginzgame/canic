use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::schema::ConfigModel,
    dto::{
        abi::v1::CanisterInitPayload, env::EnvBootstrapArgs,
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    log,
    log::Topic,
    ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
    workflow::{self},
};
use std::time::Duration;

pub fn init_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    init_nonroot_before_bootstrap(role, config, config_source, config_path, move |role| {
        workflow::runtime::init_nonroot_canister(
            role,
            payload,
            application_init_args,
            embedded_release_build_id,
        )
    });
}

pub fn init_wasm_store_before_bootstrap(
    input: FleetSubnetWasmStoreInitArgs,
    embedded_release_build_id: Option<&str>,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    init_nonroot_before_bootstrap(
        CanisterRole::WASM_STORE,
        config,
        config_source,
        config_path,
        |_| workflow::runtime::init_wasm_store_canister(input, embedded_release_build_id),
    );
}

pub fn init_local_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    env: EnvBootstrapArgs,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    init_nonroot_before_bootstrap(role, config, config_source, config_path, move |role| {
        workflow::runtime::init_local_nonroot_canister(role, env)
    });
}

pub fn init_local_nonroot_canister_with_automatic_topup_before_bootstrap(
    role: CanisterRole,
    env: EnvBootstrapArgs,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
) {
    init_nonroot_before_bootstrap(role, config, config_source, config_path, move |role| {
        workflow::runtime::init_local_nonroot_canister_with_automatic_topup(role, env)
    });
}

fn init_nonroot_before_bootstrap(
    role: CanisterRole,
    config: ConfigModel,
    config_source: &str,
    config_path: &str,
    initialize: impl FnOnce(CanisterRole) -> Result<(), crate::InternalError>,
) {
    crate::api::timer::TimerApi::initialize_nonroot_runtime_required();
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

    if let Err(err) = initialize(role) {
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

    crate::api::timer::TimerApi::defer_lifecycle_required(
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
