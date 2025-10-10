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
        CanisterSummary, root::CanisterReserve, state::CanisterStateData,
        topology::SubnetCanisterRegistry,
    },
    ops::sync::topology::root_cascade,
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
/// 2. Mark the canister as created in [`SubnetTopology`].
/// 3. Install the WASM module and flip the registry entry to "Installed".
/// 4. Cascade updated topology/state from root so children stay in sync.
pub async fn create_and_install_canister(
    ty: &CanisterType,
    parent: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // Validate type + wasm availability up-front
    Config::try_get_canister(ty)?; // must exist in config
    WasmRegistry::try_get(ty)?; // must have wasm

    // allocate PID + cycles
    let (pid, cycles) = allocate_canister(ty).await?;

    // mark Created in registry
    SubnetCanisterRegistry::create(pid, ty, parent);

    // install wasm + flip to Installed
    install_canister(pid, ty, extra_arg).await?;

    log!(Log::Ok, "⚡ create_canister: {pid} ({ty}, {cycles})");

    // Phase 3: cascade updated state from root
    root_cascade().await?;

    Ok(pid)
}

/// Uninstall and delete an existing canister, returning its recorded type.
///
/// After uninstalling the WASM code, the node is removed from
/// [`SubnetTopology`] and a root cascade is triggered so descendants learn
/// about the removal.
pub async fn uninstall_and_delete_canister(canister_pid: Principal) -> Result<String, Error> {
    // uninstall code
    uninstall_code(canister_pid).await?;

    // remove from registry
    let canister_type = if let Some(canister) = SubnetCanisterRegistry::remove(&canister_pid) {
        canister.ty.to_string()
    } else {
        String::from("unregistered")
    };

    log!(
        Log::Ok,
        "🗑️ delete_canister: {canister_pid} ({canister_type})"
    );

    // Phase 2: cascade updated state from root
    root_cascade().await?;

    Ok(canister_type)
}

//
// PHASE 0: Allocation
//

/// Allocate a canister ID and cycle balance, preferring the shared reserve.
pub async fn allocate_canister(ty: &CanisterType) -> Result<(Principal, Cycles), Error> {
    // Try pool first
    if let Some((pid, entry)) = CanisterReserve::pop_first() {
        log!(
            Log::Ok,
            "⚡ allocate_canister: reusing {} from pool ({})",
            pid,
            entry.cycles
        );

        return Ok((pid, entry.cycles));
    }

    // Fallback: create new
    let config = Config::try_get_canister(ty)?;
    let pid = raw_create_canister(config.initial_cycles.clone()).await?;

    Ok((pid, config.initial_cycles))
}

//
// PHASE 1: Creation
//

/// Create a fresh canister on the IC with the configured controllers.
pub async fn raw_create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let mut controllers = Config::try_get()?.controllers.clone();
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
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Fetch wasm
    let wasm = WasmRegistry::try_get(ty)?;

    // Canister entry
    let canister_entry = SubnetCanisterRegistry::try_get(pid)?;

    // Construct initial state
    // the view is the smaller version of the CanisterEntry
    let state = CanisterStateData {
        canister: Some(canister_entry.into()),
    };
    let parents: Vec<CanisterSummary> = SubnetCanisterRegistry::parents(pid);

    // Install code
    install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (state, parents, extra_arg),
    )
    .await?;

    // Flip to Installed
    SubnetCanisterRegistry::install(pid, wasm.module_hash())?;

    log!(
        Log::Ok,
        "⚡ install_canister: {pid} ({ty}, {:.2}KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
