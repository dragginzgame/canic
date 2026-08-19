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
    let CanisterInitPayload {
        install_id,
        release_build_id,
        authority,
        component_deployment,
    } = payload;
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
    ConfigOps::validate_protected_component_deployment(
        component_deployment.as_ref(),
        owning_component(&managed_binding),
    )?;
    let component_runtime = PreparedComponentRuntime {
        binding: managed_binding,
        deployment: *component_deployment,
    };

    // --- Phase 1: Init base systems ---
    initialize_nonroot_base(&canister_role)?;
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
    register_managed_nonroot_authority(&canister_role, authority)?;

    // Prepared managed Canisters do not start timers or application hooks.
    Ok(())
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
    init_local_nonroot_canister_with_runtime(canister_role, env, RuntimeWorkflow::start_all)
}

/// Initialize one standalone-local profile with compile-selected automatic top-up custody.
pub fn init_local_nonroot_canister_with_automatic_topup(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
) -> Result<(), InternalError> {
    init_local_nonroot_canister_with_runtime(
        canister_role,
        env,
        RuntimeWorkflow::start_all_with_automatic_topup,
    )
}

fn init_local_nonroot_canister_with_runtime(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
    start_runtime: fn() -> Result<(), InternalError>,
) -> Result<(), InternalError> {
    initialize_nonroot_base(&canister_role)?;
    FleetActivationRuntimeOps::set_standalone_local();
    EnvWorkflow::init_env_from_args(env, canister_role.clone())
        .map_err(|_err| InternalError::invariant())?;
    register_nonroot_runtime_contract(&canister_role)?;
    start_runtime()
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

    register_nonroot_runtime_contract(canister_role)
}

fn register_nonroot_runtime_contract(canister_role: &CanisterRole) -> Result<(), InternalError> {
    let app_mode = ConfigOps::app_init_mode().map_err(|_err| InternalError::invariant())?;
    FleetStateOps::init_mode(app_mode);
    let canister_cfg = ConfigOps::current_canister()?;
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(canister_role, &canister_cfg)?;
    Ok(())
}

///
/// post_upgrade_nonroot_canister
///
/// Restore runtime services for a non-root canister after stable memory init.
///

pub fn post_upgrade_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    post_upgrade_nonroot_canister_with_runtime(canister_role, RuntimeWorkflow::start_all)
}

/// Restore one managed profile with compile-selected automatic top-up custody.
pub fn post_upgrade_nonroot_canister_with_automatic_topup_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    post_upgrade_nonroot_canister_with_runtime(
        canister_role,
        RuntimeWorkflow::start_all_with_automatic_topup,
    )
}

fn post_upgrade_nonroot_canister_with_runtime(
    canister_role: CanisterRole,
    start_runtime: fn() -> Result<(), InternalError>,
) -> Result<bool, InternalError> {
    FleetActivationRuntimeOps::set_managed();
    restore_nonroot_after_upgrade(canister_role)?;
    let active = FleetActivationOps::status(false)
        .map_err(crate::ops::storage::StorageOpsError::from)?
        .phase
        == FleetActivationPhase::Active;
    if active {
        start_runtime()?;
    }
    Ok(active)
}

/// Restore one explicit standalone-local non-root after stable-memory initialization.
pub fn post_upgrade_local_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    post_upgrade_local_nonroot_canister_with_runtime(canister_role, RuntimeWorkflow::start_all)
}

/// Restore one standalone-local profile with compile-selected automatic top-up custody.
pub fn post_upgrade_local_nonroot_canister_with_automatic_topup_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    post_upgrade_local_nonroot_canister_with_runtime(
        canister_role,
        RuntimeWorkflow::start_all_with_automatic_topup,
    )
}

fn post_upgrade_local_nonroot_canister_with_runtime(
    canister_role: CanisterRole,
    start_runtime: fn() -> Result<(), InternalError>,
) -> Result<bool, InternalError> {
    FleetActivationRuntimeOps::set_standalone_local();
    restore_nonroot_after_upgrade(canister_role)?;
    start_runtime()?;
    Ok(true)
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

    Ok(())
}

const fn owning_component(binding: &ManagedCanisterBinding) -> &ComponentBinding {
    match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    }
}
