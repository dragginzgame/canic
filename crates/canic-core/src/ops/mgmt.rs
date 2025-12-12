//! Provisioning helpers for creating, installing, and tearing down canisters.
//!
//! These routines bundle the multi-phase orchestration that root performs when
//! scaling out the topology: reserving cycles, recording registry state,
//! installing WASM modules, and cascading state updates to descendants.

use crate::types::Cycles;
use crate::{
    Error,
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    config::Config,
    interface::{
        ic::{
            delete_canister as mgmt_delete_canister, deposit_cycles, get_cycles, install_code,
            uninstall_code,
        },
        prelude::*,
    },
    log::Topic,
    ops::{
        CanisterInitPayload, OpsError,
        config::ConfigOps,
        model::memory::{
            EnvOps,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            env::EnvData,
            reserve::CanisterReserveOps,
            topology::SubnetCanisterRegistryOps,
        },
        sync::state::StateBundle,
        wasm::WasmOps,
    },
};
use candid::Principal;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ProvisioningError {
    #[error(transparent)]
    Other(#[from] Error),

    #[error("install failed for {pid}: {source}")]
    InstallFailed { pid: Principal, source: Error },
}

//
// ===========================================================================
// DIRECTORY SYNC
// ===========================================================================
//

/// Rebuild AppDirectory and SubnetDirectory from the registry,
/// import them directly, and return the resulting state bundle.
/// When `updated_ty` is provided, only include the sections that list that type.
pub(crate) async fn rebuild_directories_from_registry(
    updated_ty: Option<&CanisterRole>,
) -> Result<StateBundle, Error> {
    let mut bundle = StateBundle::default();
    let cfg = Config::get();

    // did a directory change?
    let include_app = updated_ty.is_none_or(|ty| cfg.app_directory.contains(ty));
    let include_subnet = updated_ty.is_none_or(|ty| {
        ConfigOps::current_subnet()
            .map(|c| c.subnet_directory.contains(ty))
            // default to true if config is unavailable to avoid skipping a needed rebuild
            .unwrap_or(true)
    });

    if include_app {
        let app_view = AppDirectoryOps::root_build_view();
        AppDirectoryOps::import(app_view.clone());
        bundle.app_directory = Some(app_view);
    }

    if include_subnet {
        let subnet_view = SubnetDirectoryOps::root_build_view();
        SubnetDirectoryOps::import(subnet_view.clone());
        bundle.subnet_directory = Some(subnet_view);
    }

    Ok(bundle)
}

//
// ===========================================================================
// HIGH-LEVEL FLOW
// ===========================================================================
//

/// Create and install a new canister of the requested type beneath `parent`.
///
/// PHASES:
/// 1. Allocate a canister ID and cycles (preferring the reserve pool)
/// 2. Install WASM + bootstrap initial state
/// 3. Register canister in SubnetCanisterRegistry
/// 4. Cascade topology + sync directories
pub async fn create_and_install_canister(
    ty: &CanisterRole,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, ProvisioningError> {
    // must have WASM module registered
    WasmOps::try_get(ty)?;

    // Phase 1: allocation
    let pid = allocate_canister(ty).await?;

    // Phase 2: installation
    if let Err(err) = install_canister(pid, ty, parent_pid, extra_arg).await {
        return Err(ProvisioningError::InstallFailed { pid, source: err });
    }

    Ok(pid)
}

//
// ===========================================================================
// DELETION
// ===========================================================================
//

/// Delete an existing canister.
///
/// PHASES:
/// 0. Uninstall code
/// 1. Delete via management canister
/// 2. Remove from SubnetCanisterRegistry
/// 3. Cascade topology
/// 4. Sync directories
pub async fn delete_canister(
    pid: Principal,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    OpsError::require_root()?;
    let parent_pid = SubnetCanisterRegistryOps::get_parent(pid);

    // Phase 0: uninstall code
    uninstall_code(pid).await?;

    // Phase 1: delete the canister
    mgmt_delete_canister(pid).await?;

    // Phase 2: remove registry record
    let removed_entry = SubnetCanisterRegistryOps::remove(&pid);
    match &removed_entry {
        Some(c) => log!(
            Topic::CanisterLifecycle,
            Ok,
            "🗑️ delete_canister: {} ({})",
            pid,
            c.ty
        ),
        None => log!(
            Topic::CanisterLifecycle,
            Warn,
            "🗑️ delete_canister: {pid} not in registry"
        ),
    }

    Ok((removed_entry.map(|e| e.ty), parent_pid))
}

/// Uninstall code from a canister without deleting it.
pub async fn uninstall_canister(pid: Principal) -> Result<(), Error> {
    uninstall_code(pid).await?;

    log!(Topic::CanisterLifecycle, Ok, "🗑️ uninstall_canister: {pid}");

    Ok(())
}

//
// ===========================================================================
// PHASE 1 — ALLOCATION (Reserve → Create)
// ===========================================================================
//

/// Allocate a canister ID and ensure it meets the initial cycle target.
///
/// Reuses a canister from the reserve if available; otherwise creates a new one.
pub async fn allocate_canister(ty: &CanisterRole) -> Result<Principal, Error> {
    // use ConfigOps for a clean, ops-layer config lookup
    let cfg = ConfigOps::current_subnet_canister(ty)?;

    let target = cfg.initial_cycles;

    // Reuse from reserve
    if let Some((pid, _)) = CanisterReserveOps::pop_first() {
        let mut current = get_cycles(pid).await?;

        if current < target {
            let missing = target.to_u128().saturating_sub(current.to_u128());
            if missing > 0 {
                deposit_cycles(pid, missing).await?;
                current = Cycles::new(current.to_u128() + missing);

                log!(
                    Topic::CanisterReserve,
                    Ok,
                    "⚡ allocate_canister: topped up {pid} by {} to meet target {}",
                    Cycles::from(missing),
                    target
                );
            }
        }

        log!(
            Topic::CanisterReserve,
            Ok,
            "⚡ allocate_canister: reusing {pid} from pool (current {current})"
        );

        return Ok(pid);
    }

    // Create new canister
    let pid = create_canister(target).await?;
    log!(
        Topic::CanisterReserve,
        Info,
        "⚡ allocate_canister: pool empty"
    );

    Ok(pid)
}

/// Create a fresh canister on the IC with the configured controllers.
pub(crate) async fn create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self()); // root always controls

    let pid = crate::interface::ic::canister::create_canister(controllers, cycles.clone()).await?;

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ create_canister: {pid} ({cycles})"
    );

    Ok(pid)
}

//
// ===========================================================================
// PHASE 2 — INSTALLATION
// ===========================================================================
//

/// Install WASM and initial state into a new canister.
#[allow(clippy::cast_precision_loss)]
async fn install_canister(
    pid: Principal,
    ty: &CanisterRole,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Fetch and register WASM
    let wasm = WasmOps::try_get(ty)?;

    // Construct init payload
    let env = EnvData {
        prime_root_pid: Some(EnvOps::try_get_prime_root_pid()?),
        subnet_type: Some(EnvOps::try_get_subnet_type()?),
        subnet_pid: Some(EnvOps::try_get_subnet_pid()?),
        root_pid: Some(EnvOps::try_get_root_pid()?),
        canister_type: Some(ty.clone()),
        parent_pid: Some(parent_pid),
    };

    let payload = CanisterInitPayload {
        env,
        app_directory: AppDirectoryOps::export(),
        subnet_directory: SubnetDirectoryOps::export(),
    };

    let module_hash = wasm.module_hash();

    // Register before install so init hooks can observe the registry; roll back on failure.
    // otherwise if the init() tries to create a canister via root, it will panic
    SubnetCanisterRegistryOps::register(pid, ty, parent_pid, module_hash.clone());

    if let Err(err) = install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (payload, extra_arg),
    )
    .await
    {
        let removed = SubnetCanisterRegistryOps::remove(&pid);
        if removed.is_none() {
            log!(
                Topic::CanisterLifecycle,
                Warn,
                "⚠️ install_canister rollback: {pid} missing from registry after failed install"
            );
        }

        return Err(err);
    }

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ install_canister: {pid} ({ty}, {:.2} KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}

//
// ===========================================================================
