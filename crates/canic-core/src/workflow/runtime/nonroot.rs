//! Module: workflow::runtime::nonroot
//!
//! Responsibility: initialize and restore non-root canister runtime services.
//! Does not own: IC lifecycle hooks, endpoint authorization, or config schemas.
//! Boundary: lifecycle adapters call this after stable-memory restore or init input decode.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        fleet_activation::FleetActivationPhase,
        topology::{FleetDirectoryInput, SubnetDirectoryInput},
    },
    ids::CanisterRole,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::release_build::ReleaseBuildOps,
        runtime::{fleet_activation::FleetActivationRuntimeOps, memory::MemoryRegistryOps},
        storage::{
            fleet_activation::FleetActivationOps,
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            state::app::AppStateOps,
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
) -> Result<(), InternalError> {
    let CanisterInitPayload {
        fleet,
        install_id,
        release_build_id,
        env,
        fleet_directory,
        subnet_directory,
    } = payload;

    // --- Phase 1: Init base systems ---
    initialize_nonroot_base(&canister_role)?;
    FleetActivationRuntimeOps::set_managed();
    let embedded_release_build_id = ReleaseBuildOps::embedded_release_build_id()?;
    FleetActivationOps::initialize_nonroot_prepared(
        fleet,
        install_id,
        release_build_id,
        embedded_release_build_id,
    )
    .map_err(crate::ops::storage::StorageOpsError::from)?;

    // --- Phase 2: Payload registration ---
    register_nonroot_payload(&canister_role, env, fleet_directory, subnet_directory)?;

    // Prepared managed Canisters do not start timers or application hooks.
    Ok(())
}

/// Initialize one explicit standalone-local non-root without Fleet activation state.
pub fn init_local_nonroot_canister(
    canister_role: CanisterRole,
    env: EnvBootstrapArgs,
    fleet_directory: FleetDirectoryInput,
    subnet_directory: SubnetDirectoryInput,
) -> Result<(), InternalError> {
    initialize_nonroot_base(&canister_role)?;
    FleetActivationRuntimeOps::set_standalone_local();
    register_nonroot_payload(&canister_role, env, fleet_directory, subnet_directory)?;
    RuntimeWorkflow::start_all()
}

fn initialize_nonroot_base(canister_role: &CanisterRole) -> Result<(), InternalError> {
    MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })?;
    rebuild_derived_storage_indexes()?;
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    log_memory_summary();
    Ok(())
}

fn register_nonroot_payload(
    canister_role: &CanisterRole,
    env: EnvBootstrapArgs,
    fleet_directory: FleetDirectoryInput,
    subnet_directory: SubnetDirectoryInput,
) -> Result<(), InternalError> {
    EnvWorkflow::init_env_from_args(env, canister_role.clone()).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        )
    })?;

    let fleet_directory =
        AppIndexOps::filter_args_for_local_config(fleet_directory).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("Fleet Directory filter failed: {err}"),
            )
        })?;
    AppIndexOps::import_args_allow_incomplete(fleet_directory).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("Fleet Directory import failed: {err}"),
        )
    })?;
    let subnet_index =
        SubnetIndexOps::filter_args_for_local_config(subnet_directory).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("Subnet Directory filter failed: {err}"),
            )
        })?;
    SubnetIndexOps::import_args_allow_incomplete(subnet_index).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("Subnet Directory import failed: {err}"),
        )
    })?;

    let app_mode = ConfigOps::app_init_mode().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app mode init failed: {err}"),
        )
    })?;
    AppStateOps::init_mode(app_mode);
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
    FleetActivationRuntimeOps::set_managed();
    restore_nonroot_after_upgrade(canister_role)?;
    let active = FleetActivationOps::status(false)
        .map_err(crate::ops::storage::StorageOpsError::from)?
        .phase
        == FleetActivationPhase::Active;
    if active {
        RuntimeWorkflow::start_all()?;
    }
    Ok(active)
}

/// Restore one explicit standalone-local non-root after stable-memory initialization.
pub fn post_upgrade_local_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
) -> Result<bool, InternalError> {
    FleetActivationRuntimeOps::set_standalone_local();
    restore_nonroot_after_upgrade(canister_role)?;
    RuntimeWorkflow::start_all()?;
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
    let canister_cfg = ConfigOps::current_canister().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("current canister config unavailable during post-upgrade runtime init: {err}"),
        )
    })?;
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&canister_role, &canister_cfg)?;

    Ok(())
}
