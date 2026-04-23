use crate::{
    InternalError, InternalErrorOrigin,
    dto::abi::v1::CanisterInitPayload,
    ids::CanisterRole,
    ops::{
        config::ConfigOps,
        runtime::memory::{MemoryRegistryInitSummary, MemoryRegistryOps},
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            state::app::AppStateOps,
        },
    },
    workflow::{env::EnvWorkflow, prelude::*},
};

use super::{RuntimeWorkflow, auth::RuntimeAuthWorkflow, log_memory_summary};

///
/// init_nonroot_canister
///
/// Restore runtime state for a non-root canister during `init`.
///

pub fn init_nonroot_canister(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
) -> Result<(), InternalError> {
    init_nonroot_canister_internal(canister_role, payload, false)
}

pub fn init_nonroot_canister_with_attestation_cache(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
) -> Result<(), InternalError> {
    init_nonroot_canister_internal(canister_role, payload, true)
}

// Initialize a non-root canister and start only the runtime services it needs.
fn init_nonroot_canister_internal(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
    with_attestation_cache: bool,
) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
    let memory_summary = MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })?;
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    log_memory_summary(&memory_summary);

    // --- Phase 2: Payload registration ---
    EnvWorkflow::init_env_from_args(payload.env, canister_role.clone()).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        )
    })?;

    AppIndexOps::import_args_allow_incomplete(payload.app_index).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app index import failed: {err}"),
        )
    })?;
    SubnetIndexOps::import_args_allow_incomplete(payload.subnet_index).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("subnet index import failed: {err}"),
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
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&canister_role, &canister_cfg)?;

    // --- Phase 3: Service startup ---
    if with_attestation_cache {
        RuntimeWorkflow::start_all_with_attestation_cache();
    } else {
        RuntimeWorkflow::start_all();
    }

    Ok(())
}

///
/// post_upgrade_nonroot_canister
///
/// Restore runtime services for a non-root canister after stable memory init.
///

pub fn post_upgrade_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
    memory_summary: MemoryRegistryInitSummary,
) {
    post_upgrade_nonroot_canister_after_memory_init_internal(canister_role, memory_summary, false);
}

pub fn post_upgrade_nonroot_canister_after_memory_init_with_attestation_cache(
    canister_role: CanisterRole,
    memory_summary: MemoryRegistryInitSummary,
) {
    post_upgrade_nonroot_canister_after_memory_init_internal(canister_role, memory_summary, true);
}

// Restore post-upgrade runtime services for a non-root canister.
fn post_upgrade_nonroot_canister_after_memory_init_internal(
    canister_role: CanisterRole,
    memory_summary: MemoryRegistryInitSummary,
    with_attestation_cache: bool,
) {
    crate::log::set_ready();
    crate::log!(
        Topic::Init,
        Info,
        "🏁 post_upgrade_nonroot_canister: {}",
        canister_role
    );
    log_memory_summary(&memory_summary);

    // --- Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.
    let canister_cfg = ConfigOps::current_canister().unwrap_or_else(|err| {
        panic!("current canister config unavailable during post-upgrade runtime init: {err}")
    });
    RuntimeAuthWorkflow::ensure_nonroot_crypto_contract(&canister_role, &canister_cfg)
        .unwrap_or_else(|err| panic!("non-root delegated auth runtime contract failed: {err}"));

    // --- Phase 3: Service startup ---
    if with_attestation_cache {
        RuntimeWorkflow::start_all_with_attestation_cache();
    } else {
        RuntimeWorkflow::start_all();
    }
}
