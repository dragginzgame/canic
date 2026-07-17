//! Module: workflow::runtime::nonroot
//!
//! Responsibility: initialize and restore non-root canister runtime services.
//! Does not own: IC lifecycle hooks, endpoint authorization, or config schemas.
//! Boundary: lifecycle adapters call this after stable-memory restore or init input decode.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::abi::v1::CanisterInitPayload,
    ids::CanisterRole,
    log::Topic,
    ops::{
        config::ConfigOps,
        runtime::memory::MemoryRegistryOps,
        storage::{
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
    with_role_attestation_refresh: bool,
) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
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

    // --- Phase 2: Payload registration ---
    EnvWorkflow::init_env_from_args(payload.env, canister_role.clone()).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        )
    })?;

    let app_index =
        AppIndexOps::filter_args_for_local_config(payload.app_index).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("app index filter failed: {err}"),
            )
        })?;
    AppIndexOps::import_args_allow_incomplete(app_index).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app index import failed: {err}"),
        )
    })?;
    let subnet_index =
        SubnetIndexOps::filter_args_for_local_config(payload.subnet_index).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("subnet index filter failed: {err}"),
            )
        })?;
    SubnetIndexOps::import_args_allow_incomplete(subnet_index).map_err(|err| {
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
    if with_role_attestation_refresh {
        RuntimeWorkflow::start_all_with_role_attestation_refresh();
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
    with_role_attestation_refresh: bool,
) -> Result<(), InternalError> {
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

    // --- Phase 3: Service startup ---
    if with_role_attestation_refresh {
        RuntimeWorkflow::start_all_with_role_attestation_refresh();
    } else {
        RuntimeWorkflow::start_all();
    }

    Ok(())
}
