//! Module: workflow::runtime::nonroot
//!
//! Responsibility: initialize and restore non-root canister runtime services.
//! Does not own: IC lifecycle hooks, endpoint authorization, or config schemas.
//! Boundary: lifecycle adapters call this after stable-memory restore or init input decode.

use crate::{
    InternalError,
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        env::EnvBootstrapArgs,
        fleet_activation::FleetActivationPhase,
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::{CanisterRole, ComponentBinding, ManagedCanisterBinding},
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{IcOps, release_build::ReleaseBuildOps},
        runtime::{fleet_activation::FleetActivationRuntimeOps, memory::MemoryRegistryOps},
        storage::{
            fleet_activation::{FleetActivationOps, PreparedComponentRuntime},
            state::fleet::FleetStateOps,
        },
    },
    workflow::{
        env::EnvWorkflow,
        fleet_admission_projection::FleetAdmissionProjectionWorkflow,
        runtime::{
            RuntimeWorkflow, auth::RuntimeAuthWorkflow, log_memory_summary,
            rebuild_derived_storage_indexes,
        },
    },
};

///
/// init_nonroot_canister
///
/// Restore runtime state for a non-root canister during `init`.
///

pub fn init_nonroot_canister(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
) -> Result<(), InternalError> {
    prepare_managed_nonroot(
        &canister_role,
        payload,
        application_init_args,
        embedded_release_build_id,
        false,
    )?;
    register_nonroot_runtime_contract(&canister_role)
}

/// Initialize the compile-selected Fleet admission projection before application startup.
pub fn init_nonroot_canister_with_fleet_admission(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
) -> Result<(), InternalError> {
    let admission = prepare_managed_nonroot(
        &canister_role,
        payload,
        application_init_args,
        embedded_release_build_id,
        true,
    )?
    .ok_or_else(InternalError::invariant)?;
    FleetAdmissionProjectionWorkflow::initialize(admission)?;
    register_nonroot_runtime_contract(&canister_role)
}

fn prepare_managed_nonroot(
    canister_role: &CanisterRole,
    payload: CanisterInitPayload,
    application_init_args: Option<Vec<u8>>,
    embedded_release_build_id: Option<&str>,
    selected_admission: bool,
) -> Result<Option<crate::ids::FleetAdmissionProjection>, InternalError> {
    let CanisterInitPayload {
        fixture,
        install_id,
        release_build_id,
        authority,
        component_deployment,
        admission,
    } = payload;
    let admission = validate_fleet_admission_payload(
        selected_admission,
        ConfigOps::role_uses_fleet_admission(canister_role)?,
        admission,
    )?;
    let fleet = match &authority {
        CanisterInitAuthority::Component { binding, .. } => binding.authority.binding.fleet.clone(),
        CanisterInitAuthority::ComponentChild { binding, .. } => {
            binding.component.authority.binding.fleet.clone()
        }
    };
    let managed_binding = match &authority {
        CanisterInitAuthority::Component { binding, .. } => {
            ManagedCanisterBinding::Component(binding.clone())
        }
        CanisterInitAuthority::ComponentChild { binding, .. } => {
            ManagedCanisterBinding::ComponentChild(binding.clone())
        }
    };
    if let Some(assignment) = &fixture {
        let target = match &assignment.grant.binding.target {
            ManagedCanisterBinding::Component(binding) => binding.canister_id,
            ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
        };
        if target != IcOps::canister_self() {
            return Err(InternalError::conflict());
        }
    }
    ConfigOps::validate_protected_component_deployment(
        component_deployment.as_ref(),
        owning_component(&managed_binding),
    )?;
    let component_runtime = PreparedComponentRuntime {
        fixture,
        binding: managed_binding,
        deployment: *component_deployment,
    };

    // --- Phase 1: Init base systems ---
    initialize_nonroot_base(canister_role)?;
    FleetActivationRuntimeOps::set_managed();
    let embedded_release_build_id =
        ReleaseBuildOps::embedded_release_build_id(embedded_release_build_id)?;
    FleetActivationOps::initialize_nonroot_prepared(
        fleet,
        install_id,
        release_build_id,
        embedded_release_build_id,
        Some(component_runtime),
        application_init_args,
    )
    .map_err(crate::ops::storage::StorageOpsError::from)?;

    // --- Phase 2: Payload registration ---
    register_managed_nonroot_authority(canister_role, authority)?;

    // Prepared managed Canisters do not start timers or application hooks.
    Ok(admission)
}

/// Initialize one host-installed sibling Wasm Store with reciprocal root authority.
pub fn init_wasm_store_canister(
    input: FleetSubnetWasmStoreInitArgs,
    embedded_release_build_id: Option<&str>,
) -> Result<(), InternalError> {
    let canister_role = CanisterRole::WASM_STORE;
    let authority = input.authority.clone();
    initialize_nonroot_base(&canister_role)?;
    FleetActivationRuntimeOps::set_managed();
    let embedded_release_build_id =
        ReleaseBuildOps::embedded_release_build_id(embedded_release_build_id)?;
    FleetActivationOps::initialize_wasm_store_prepared(
        input,
        embedded_release_build_id,
        IcOps::canister_self(),
    )
    .map_err(crate::ops::storage::StorageOpsError::from)?;

    let root = authority.fleet_subnet_root;
    let env = EnvBootstrapArgs {
        fleet_subnet_root_pid: Some(root),
        component_spec: None,
        subnet_pid: Some(*authority.placement_subnet.as_principal()),
        root_pid: Some(root),
        canister_role: Some(canister_role.clone()),
        parent_pid: Some(root),
    };
    EnvWorkflow::init_env_from_args(env, canister_role.clone())
        .map_err(|_err| InternalError::invariant())?;
    register_nonroot_runtime_contract(&canister_role)
}

/// Initialize one explicit standalone-local non-root without Fleet activation state.
pub fn init_local_nonroot_canister(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
) -> Result<(), InternalError> {
    initialize_local_nonroot(canister_role, env)?;
    RuntimeWorkflow::start_all()
}

/// Initialize one standalone-local profile with compile-selected automatic top-up custody.
pub fn init_local_nonroot_canister_with_automatic_topup(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
) -> Result<(), InternalError> {
    initialize_local_nonroot(canister_role, env)?;
    RuntimeWorkflow::start_all_with_automatic_topup()
}

fn initialize_local_nonroot(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
) -> Result<(), InternalError> {
    initialize_nonroot_base(&canister_role)?;
    FleetActivationRuntimeOps::set_standalone_local();
    EnvWorkflow::init_env_from_args(env, canister_role.clone())
        .map_err(|_err| InternalError::invariant())?;
    register_nonroot_runtime_contract(&canister_role)
}

fn initialize_nonroot_base(canister_role: &CanisterRole) -> Result<(), InternalError> {
    MemoryRegistryOps::bootstrap_registry().map_err(|_err| InternalError::invariant())?;
    rebuild_derived_storage_indexes()?;
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    log_memory_summary();
    Ok(())
}

fn register_managed_nonroot_authority(
    canister_role: &CanisterRole,
    authority: CanisterInitAuthority,
) -> Result<(), InternalError> {
    match authority {
        CanisterInitAuthority::Component { root, binding } => {
            EnvWorkflow::init_component(&root, binding, canister_role)?;
        }
        CanisterInitAuthority::ComponentChild { root, binding } => {
            EnvWorkflow::init_component_child(&root, binding, canister_role)?;
        }
    }

    Ok(())
}

fn register_nonroot_runtime_contract(canister_role: &CanisterRole) -> Result<(), InternalError> {
    let app_mode = ConfigOps::app_init_mode().map_err(|_err| InternalError::invariant())?;
    FleetStateOps::init_mode(app_mode);
    let canister_cfg = ConfigOps::current_canister()?;
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(canister_role, &canister_cfg)?;
    RuntimeAuthWorkflow::reconcile_local_application_authority()?;
    Ok(())
}

///
/// post_upgrade_nonroot_canister
///
/// Restore runtime services for a non-root canister after stable memory init.
///

pub fn post_upgrade_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
    embedded_release_build_id: Option<&str>,
) -> Result<bool, InternalError> {
    let active = restore_managed_nonroot(canister_role, embedded_release_build_id, false)?;
    if active {
        RuntimeWorkflow::start_all()?;
    }
    Ok(active)
}

/// Restore one managed profile with compile-selected automatic top-up custody.
pub fn post_upgrade_nonroot_canister_with_automatic_topup_after_memory_init(
    canister_role: CanisterRole,
    embedded_release_build_id: Option<&str>,
) -> Result<bool, InternalError> {
    let active = restore_managed_nonroot(canister_role, embedded_release_build_id, false)?;
    if active {
        RuntimeWorkflow::start_all_with_automatic_topup()?;
    }
    Ok(active)
}

/// Restore compile-selected Fleet admission before starting the selected runtime services.
pub fn post_upgrade_nonroot_canister_with_fleet_admission_after_memory_init(
    canister_role: CanisterRole,
    embedded_release_build_id: Option<&str>,
) -> Result<bool, InternalError> {
    let active = restore_managed_nonroot(canister_role, embedded_release_build_id, true)?;
    FleetAdmissionProjectionWorkflow::restore()?;
    if active {
        RuntimeWorkflow::start_all()?;
    }
    Ok(active)
}

/// Restore compile-selected Fleet admission before starting the selected runtime services.
pub fn post_upgrade_nonroot_canister_with_automatic_topup_and_fleet_admission_after_memory_init(
    canister_role: CanisterRole,
    embedded_release_build_id: Option<&str>,
) -> Result<bool, InternalError> {
    let active = restore_managed_nonroot(canister_role, embedded_release_build_id, true)?;
    FleetAdmissionProjectionWorkflow::restore()?;
    if active {
        RuntimeWorkflow::start_all_with_automatic_topup()?;
    }
    Ok(active)
}

fn restore_managed_nonroot(
    canister_role: CanisterRole,
    embedded_release_build_id: Option<&str>,
    selected_admission: bool,
) -> Result<bool, InternalError> {
    FleetActivationRuntimeOps::set_managed();
    let embedded_release_build_id =
        ReleaseBuildOps::embedded_release_build_id(embedded_release_build_id)?;
    FleetActivationOps::require_release_build(embedded_release_build_id)
        .map_err(crate::ops::storage::StorageOpsError::from)?;
    let enrolled = if canister_role.is_wasm_store() {
        false
    } else {
        ConfigOps::role_uses_fleet_admission(&canister_role)?
    };
    validate_fleet_admission_selection(selected_admission, enrolled)?;
    restore_nonroot_after_upgrade(canister_role)?;
    let active = FleetActivationOps::status(false)
        .map_err(crate::ops::storage::StorageOpsError::from)?
        .phase
        == FleetActivationPhase::Active;
    Ok(active)
}

/// Restore one explicit standalone-local non-root after stable-memory initialization.
pub fn post_upgrade_local_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    restore_local_nonroot(canister_role)?;
    RuntimeWorkflow::start_all()?;
    Ok(true)
}

/// Restore one standalone-local profile with compile-selected automatic top-up custody.
pub fn post_upgrade_local_nonroot_canister_with_automatic_topup_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    restore_local_nonroot(canister_role)?;
    RuntimeWorkflow::start_all_with_automatic_topup()?;
    Ok(true)
}

fn restore_local_nonroot(canister_role: CanisterRole) -> Result<(), InternalError> {
    FleetActivationRuntimeOps::set_standalone_local();
    restore_nonroot_after_upgrade(canister_role)
}

fn restore_nonroot_after_upgrade(canister_role: CanisterRole) -> Result<(), InternalError> {
    rebuild_derived_storage_indexes()?;
    crate::log::set_ready();
    crate::log!(
        Topic::Init,
        Info,
        "🏁 post_upgrade_nonroot_canister: {}",
        canister_role
    );
    log_memory_summary();

    // --- Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.
    let canister_cfg = ConfigOps::current_canister().map_err(|_err| InternalError::invariant())?;
    if !FleetActivationRuntimeOps::is_standalone_local() && !canister_role.is_wasm_store() {
        let binding = crate::ops::runtime::env::EnvOps::managed_binding()?;
        let deployment = FleetActivationOps::component_deployment()
            .map_err(crate::ops::storage::StorageOpsError::from)?;
        ConfigOps::validate_protected_component_deployment(
            &deployment,
            owning_component(&binding),
        )?;
    }
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&canister_role, &canister_cfg)?;
    RuntimeAuthWorkflow::reconcile_local_application_authority()?;

    Ok(())
}

const fn owning_component(binding: &ManagedCanisterBinding) -> &ComponentBinding {
    match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    }
}

const fn validate_fleet_admission_selection(
    selected: bool,
    enrolled: bool,
) -> Result<(), InternalError> {
    if selected == enrolled {
        Ok(())
    } else {
        Err(InternalError::invariant())
    }
}

fn validate_fleet_admission_payload(
    selected: bool,
    enrolled: bool,
    admission: Option<crate::ids::FleetAdmissionProjection>,
) -> Result<Option<crate::ids::FleetAdmissionProjection>, InternalError> {
    validate_fleet_admission_selection(selected, enrolled)?;
    match (enrolled, admission) {
        (true, Some(admission)) => Ok(Some(admission)),
        (false, None) => Ok(None),
        (true, None) | (false, Some(_)) => Err(InternalError::invariant()),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_fleet_admission_payload;

    #[test]
    fn managed_init_projection_matches_selected_and_declared_enrollment() {
        let projection = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        for selected in [false, true] {
            for enrolled in [false, true] {
                for present in [false, true] {
                    let result = validate_fleet_admission_payload(
                        selected,
                        enrolled,
                        present.then(|| projection.clone()),
                    );
                    if selected == enrolled && enrolled == present {
                        assert_eq!(result.unwrap().is_some(), present);
                    } else {
                        assert_eq!(
                            result.unwrap_err().code(),
                            crate::diagnostics::codes::STATE_INVALID
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn restored_admission_selection_must_match_compiled_authority() {
        for selected in [false, true] {
            for enrolled in [false, true] {
                let result = super::validate_fleet_admission_selection(selected, enrolled);
                if selected == enrolled {
                    result.unwrap();
                } else {
                    assert_eq!(
                        result.unwrap_err().code(),
                        crate::diagnostics::codes::STATE_INVALID
                    );
                }
            }
        }
    }
}
