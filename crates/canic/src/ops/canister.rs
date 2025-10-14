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
    memory::{Env, env::EnvData, root::reserve::CanisterReserve, topology::SubnetCanisterRegistry},
    ops::{
        context::cfg_current_subnet,
        directory::{add_to_directories, remove_from_directories},
        sync::topology::root_cascade_topology,
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
/// 2. Install the WASM module and bootstrap the initial state payload.
/// 3. Record the canister in [`SubnetCanisterRegistry`] and update directory membership.
/// 4. Cascade refreshed topology (and directory state, if applicable) so children stay in sync.
pub async fn create_and_install_canister(
    ty: &CanisterType,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    let subnet_cfg = cfg_current_subnet()?;

    // Validate upfront
    subnet_cfg.try_get_canister(ty)?; // must exist in config
    WasmRegistry::try_get(ty)?; // must have wasm

    // Phase 1: allocate and install
    let pid = allocate_canister(ty).await?;
    install_canister(pid, ty, parent_pid, extra_arg).await?;

    // Phase 2: cascade topology
    root_cascade_topology().await?;

    // Phase 3: update directories (this will cause a cascade)
    add_to_directories(ty, pid).await?;

    Ok(pid)
}

///
/// Uninstall and delete an existing canister.
///
/// After uninstalling the WASM code, the node is removed from
/// [`SubnetCanisterRegistry`] and a root cascade is triggered so descendants learn
/// about the removal.
///

pub async fn uninstall_and_delete_canister(pid: Principal) -> Result<(), Error> {
    // Phase 0: uninstall code
    uninstall_code(pid).await?;

    // Phase 1: remove from registry
    let Some(canister) = SubnetCanisterRegistry::remove(&pid) else {
        log!(Log::Warn, "🗑️ delete_canister: {pid} not in registry");

        return Ok(());
    };

    log!(Log::Ok, "🗑️ delete_canister: {} ({})", pid, canister.ty);

    // Phase 2: cascade
    root_cascade_topology().await?;

    // Phase 3: update directory if it existed
    remove_from_directories(&canister.ty).await?;

    Ok(())
}

//
// PHASE 1: Allocation or Creation
//

/// Allocate a canister ID and cycle balance, preferring the shared reserve.
pub async fn allocate_canister(ty: &CanisterType) -> Result<Principal, Error> {
    if let Some((pid, entry)) = CanisterReserve::pop_first() {
        log!(
            Log::Ok,
            "⚡ allocate_canister: reusing {} from pool ({})",
            pid,
            entry.cycles
        );

        Ok(pid)
    } else {
        let cfg = cfg_current_subnet()?.try_get_canister(ty)?;
        let pid = create_canister(cfg.initial_cycles.clone()).await?;
        log!(Log::Info, "⚡ allocate_canister: pool empty");

        Ok(pid)
    }
}

/// Create a fresh canister on the IC with the configured controllers.
pub(crate) async fn create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self()); // root always controls

    let pid = crate::interface::ic::create_canister(controllers, cycles.clone()).await?;
    log!(Log::Ok, "⚡ create_canister: {pid} ({cycles})");

    Ok(pid)
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
    // Construct initial state
    // the view is the smaller version of the CanisterEntry
    let env = EnvData {
        prime_root_pid: Env::get_prime_root_pid(),
        subnet_type: Env::get_subnet_type(),
        subnet_pid: Env::get_subnet_pid(),
        root_pid: Env::get_root_pid(),
        canister_type: Some(ty.clone()),
        parent_pid: Some(parent_pid),
    };

    // Fetch WASM and Install code
    let wasm = WasmRegistry::try_get(ty)?;
    install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (env, extra_arg),
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
