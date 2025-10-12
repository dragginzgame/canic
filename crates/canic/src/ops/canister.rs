//! Provisioning helpers for creating, installing, and tearing down canisters.
//!
//! These routines bundle the multi-phase orchestration that root performs when
//! scaling out the topology: reserving cycles, recording registry state,
//! installing WASM modules, and cascading state updates to descendants.

use crate::{
    Error,
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    config::Config,
    interface::{
        ic::{install_code, uninstall_code},
        prelude::*,
    },
    memory::{
        CanisterSummary, Env, directory::SubnetDirectory, env::EnvData,
        root::reserve::CanisterReserve, topology::SubnetCanisterRegistry,
    },
    ops::{
        context::cfg_current_subnet,
        sync::{
            state::{StateBundle, root_cascade_state},
            topology::root_cascade_topology,
        },
    },
    state::wasm::WasmRegistry,
};
use candid::Principal;

//
// HIGH-LEVEL FLOW
//

/// Create and install a new canister of the requested type beneath `parent`.
///
/// The helper performs the following phases:
/// 1. Allocate a canister ID and cycles (preferring the reserve pool).
/// 2. Mark the canister as created in [`SubnetCanisterRegistry`].
/// 3. Install the WASM module and flip the registry entry to "Installed".
/// 4. Cascade updated topology/state from root so children stay in sync.
pub async fn create_and_install_canister(
    ty: &CanisterType,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    let subnet_cfg = cfg_current_subnet()?;

    // Validate type + wasm availability up-front
    subnet_cfg.try_get_canister(ty)?; // must exist in config
    WasmRegistry::try_get(ty)?; // must have wasm

    // allocate PID + cycles
    let (pid, cycles) = allocate_canister(ty).await?;

    // install canister
    install_canister(pid, ty, parent_pid, extra_arg).await?;

    log!(Log::Ok, "⚡ create_canister: {pid} ({ty}, {cycles})");

    // cascade
    root_cascade_topology().await?;
    if subnet_cfg.directory.contains(ty) {
        let bundle = StateBundle::subnet_directory();
        root_cascade_state(bundle).await?;
    }

    Ok(pid)
}

///
/// Uninstall and delete an existing canister, returning its recorded type.
///
/// After uninstalling the WASM code, the node is removed from
/// [`SubnetCanisterRegistry`] and a root cascade is triggered so descendants learn
/// about the removal.
///

pub async fn uninstall_and_delete_canister(canister_pid: Principal) -> Result<(), Error> {
    // Phase 0: uninstall code
    uninstall_code(canister_pid).await?;

    // Phase 1: remove from registry
    let Some(canister) = SubnetCanisterRegistry::remove(&canister_pid) else {
        log!(
            Log::Warn,
            "🗑️ delete_canister: {canister_pid} not in registry"
        );

        return Ok(());
    };

    log!(
        Log::Ok,
        "🗑️ delete_canister: {} ({})",
        canister_pid,
        canister.ty
    );

    // Phase 2: cascade
    root_cascade_topology().await?;

    // Phase 3: update directory if it existed
    if SubnetDirectory::remove(&canister.ty).is_some() {
        let bundle = StateBundle::subnet_directory();
        root_cascade_state(bundle).await?;
    }

    Ok(())
}

//
// PHASE 0: Allocation
//

/// Allocate a canister ID and cycle balance, preferring the shared reserve.
pub async fn allocate_canister(ty: &CanisterType) -> Result<(Principal, Cycles), Error> {
    let (pid, cycles) = if let Some((pid, entry)) = CanisterReserve::pop_first() {
        log!(
            Log::Ok,
            "⚡ allocate_canister: reusing {} from pool ({})",
            pid,
            entry.cycles
        );

        (pid, entry.cycles)
    } else {
        let cfg = cfg_current_subnet()?.try_get_canister(ty)?;
        let pid = raw_create_canister(cfg.initial_cycles.clone()).await?;

        (pid, cfg.initial_cycles)
    };

    Ok((pid, cycles))
}

//
// PHASE 1: Creation
//

/// Create a fresh canister on the IC with the configured controllers.
pub async fn raw_create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self()); // root always controls

    crate::interface::ic::create_canister(controllers, cycles).await
}

//
// PHASE 2: Installation
//

/// Install code and initial state into a new canister.
#[allow(clippy::cast_precision_loss)]
async fn install_canister(
    pid: Principal,
    ty: &CanisterType,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Register directory membership first, if applicable.
    let subnet_cfg = cfg_current_subnet()?;
    if subnet_cfg.directory.contains(ty) {
        SubnetDirectory::register(ty, pid)?;
    }

    // Construct initial state
    // the view is the smaller version of the CanisterEntry
    let env = EnvData {
        prime_root_pid: Env::get_prime_root_pid(),
        subnet_type: Env::get_subnet_type(),
        subnet_pid: Env::get_subnet_pid(),
        root_pid: Env::get_root_pid(),
        canister_type: Some(ty.clone()),
    };
    let parents: Vec<CanisterSummary> = SubnetCanisterRegistry::parents(pid);

    // Fetch WASM and Install code
    let wasm = WasmRegistry::try_get(ty)?;
    install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (env, parents, extra_arg),
    )
    .await?;

    // Register in topology registry after successful install
    SubnetCanisterRegistry::register(pid, ty, parent_pid, wasm.module_hash());

    log!(
        Log::Ok,
        "⚡ install_canister: {pid} ({ty}, {:.2}KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
