use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap,
    config::RoleRuntimeAuthority,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    ops::runtime::{
        bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
        env::EnvOps,
    },
    workflow,
};
use canic_contracts::ids::CanisterRole;

pub fn post_upgrade_wasm_store_before_bootstrap(
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(CanisterRole::WASM_STORE, authority, move |_role| {
        workflow::runtime::post_upgrade_wasm_store_after_memory_init(embedded_release_build_id)
    })
}

pub fn post_upgrade_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::post_upgrade_nonroot_canister_after_memory_init(
            role,
            embedded_release_build_id,
        )
    })
}

pub fn post_upgrade_nonroot_canister_with_fleet_admission_before_bootstrap(
    role: CanisterRole,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::post_upgrade_nonroot_canister_with_fleet_admission_after_memory_init(
            role,
            embedded_release_build_id,
        )
    })
}

pub fn post_upgrade_nonroot_canister_with_automatic_topup_before_bootstrap(
    role: CanisterRole,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::post_upgrade_nonroot_canister_with_automatic_topup_after_memory_init(
            role,
            embedded_release_build_id,
        )
    })
}

pub fn post_upgrade_nonroot_canister_with_automatic_topup_and_fleet_admission_before_bootstrap(
    role: CanisterRole,
    embedded_release_build_id: Option<&str>,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(role, authority, move |role| {
        workflow::runtime::post_upgrade_nonroot_canister_with_automatic_topup_and_fleet_admission_after_memory_init(
            role,
            embedded_release_build_id,
        )
    })
}

pub fn post_upgrade_local_nonroot_canister_before_bootstrap(
    role: CanisterRole,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(
        role,
        authority,
        workflow::runtime::post_upgrade_local_nonroot_canister_after_memory_init,
    )
}

pub fn post_upgrade_local_nonroot_canister_with_automatic_topup_before_bootstrap(
    role: CanisterRole,
    authority: RoleRuntimeAuthority,
) -> bool {
    post_upgrade_nonroot_before_bootstrap(
        role,
        authority,
        workflow::runtime::post_upgrade_local_nonroot_canister_with_automatic_topup_after_memory_init,
    )
}

fn post_upgrade_nonroot_before_bootstrap(
    role: CanisterRole,
    authority: RoleRuntimeAuthority,
    restore: impl FnOnce(CanisterRole) -> Result<bool, crate::InternalError>,
) -> bool {
    crate::api::timer::TimerApi::initialize_shared_runtime_required();
    LifecycleMetricsApi::record_runtime(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Started,
    );

    if let Err(err) = bootstrap::init_role_runtime_authority(&role, authority) {
        LifecycleMetricsApi::record_runtime(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricOutcome::Failed,
        );
        lifecycle_trap(
            LifecyclePhase::PostUpgrade,
            format!("runtime authority init failed: {err}"),
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

    workflow::bootstrap::timer::schedule(LifecyclePhase::PostUpgrade)
        .unwrap_or_else(|error| lifecycle_trap(LifecyclePhase::PostUpgrade, error));
}
