use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::RoleRuntimeAuthority,
    dto::{
        abi::v1::CanisterInitPayload, env::EnvBootstrapArgs,
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap, retryable_nonroot_bootstrap_error},
    log,
    log::Topic,
    ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
    workflow::{self},
};
use std::time::Duration;

const MAX_NONROOT_BOOTSTRAP_ATTEMPTS: u32 = 64;

pub fn init_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) {
    init_nonroot_before_bootstrap(role, authority, move |role| {
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
    authority: RoleRuntimeAuthority,
) {
    init_nonroot_before_bootstrap(CanisterRole::WASM_STORE, authority, |_| {
        workflow::runtime::init_wasm_store_canister(input, embedded_release_build_id)
    });
}

pub fn init_local_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    env: EnvBootstrapArgs,
    authority: RoleRuntimeAuthority,
) {
    init_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::init_local_nonroot_canister(role, env)
    });
}

pub fn init_local_nonroot_canister_with_automatic_topup_before_bootstrap(
    role: CanisterRole,
    env: EnvBootstrapArgs,
    authority: RoleRuntimeAuthority,
) {
    init_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::init_local_nonroot_canister_with_automatic_topup(role, env)
    });
}

fn init_nonroot_before_bootstrap(
    role: CanisterRole,
    authority: RoleRuntimeAuthority,
    initialize: impl FnOnce(CanisterRole) -> Result<(), crate::InternalError>,
) {
    crate::api::timer::TimerApi::initialize_nonroot_runtime_required();
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_role_runtime_authority(&role, authority) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("runtime authority init failed: {err}"),
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
    if !BootstrapStatusOps::try_schedule_nonroot_init() {
        return;
    }
    LifecycleMetricsApi::record_bootstrap(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Scheduled,
    );
    schedule_init_nonroot_bootstrap_after(0, Duration::ZERO);
}

fn schedule_init_nonroot_bootstrap_after(attempt: u32, delay: Duration) {
    crate::api::timer::TimerApi::defer_lifecycle_required(
        delay,
        "canic:bootstrap:init_nonroot_canister",
        async move {
            BootstrapStatusOps::set_phase(BootstrapPhaseLabel::NONROOT_INIT);
            LifecycleMetricsApi::record_bootstrap(
                LifecycleMetricPhase::Init,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Started,
            );
            if let Err(err) = workflow::bootstrap::nonroot::bootstrap_init_nonroot_canister().await
            {
                let next_attempt = attempt.saturating_add(1);
                if retryable_nonroot_bootstrap_error(&err)
                    && next_attempt < MAX_NONROOT_BOOTSTRAP_ATTEMPTS
                {
                    BootstrapStatusOps::set_phase(
                        BootstrapPhaseLabel::NONROOT_INIT_WAITING_AUTHORITY,
                    );
                    log!(
                        Topic::Init,
                        Warn,
                        "non-root bootstrap waiting for managed authority (attempt {next_attempt}/{MAX_NONROOT_BOOTSTRAP_ATTEMPTS}): {err}"
                    );
                    schedule_init_nonroot_bootstrap_after(
                        next_attempt,
                        bootstrap_retry_delay(next_attempt),
                    );
                    return;
                }
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

const fn bootstrap_retry_delay(attempt: u32) -> Duration {
    let exponent = if attempt > 4 { 4 } else { attempt };
    Duration::from_millis(250_u64.saturating_mul(1_u64 << exponent))
}
