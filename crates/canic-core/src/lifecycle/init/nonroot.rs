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
    lifecycle::{LifecyclePhase, lifecycle_trap},
    ops::runtime::bootstrap::BootstrapStatusOps,
    workflow,
};
use canic_contracts::ids::CanisterRole;

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

pub fn init_nonroot_canister_with_fleet_admission_before_bootstrap(
    role: CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) {
    init_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::init_nonroot_canister_with_fleet_admission(
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
    crate::api::timer::TimerApi::initialize_shared_runtime_required();
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
    workflow::bootstrap::timer::schedule(LifecyclePhase::Init)
        .unwrap_or_else(|error| lifecycle_trap(LifecyclePhase::Init, error));
}
