use crate::{
    Error,
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    config::Config,
    interface::{
        ic::{install_code, uninstall_code},
        prelude::*,
    },
    memory::{
        CanisterSummary, root::CanisterReserve, state::CanisterStateData, topology::SubnetTopology,
    },
    ops::sync::topology::root_cascade,
    state::wasm::WasmRegistry,
};
use candid::Principal;

//
// HIGH-LEVEL FLOW
//

/// Create + install a new canister of given type under parent.
/// Phases:
///   0. Allocate PID + cycles
///   1. Mark Created in registry
///   2. Install wasm + flip to Installed
///   3. Cascade updated state from root
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
    SubnetTopology::create(pid, ty, parent);

    // install wasm + flip to Installed
    install_canister(pid, ty, extra_arg).await?;

    log!(Log::Ok, "⚡ create_canister: {pid} ({ty}, {cycles})");

    // Phase 3: cascade updated state from root
    root_cascade().await?;

    Ok(pid)
}

/// Uninstall + delete an existing canister.
/// Phases:
///   0. Uninstall wasm code
///   1. Remove entry from registry
///   2. Cascade updated state from root
pub async fn uninstall_and_delete_canister(canister_pid: Principal) -> Result<String, Error> {
    // uninstall code
    uninstall_code(canister_pid).await?;

    // remove from registry
    let canister_type = if let Some(canister) = SubnetTopology::remove(&canister_pid) {
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

/// Allocate a canister ID + cycles, preferring the pool.
/// Returns (pid, cycles).
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

/// Create a fresh canister on IC with given cycles + controllers.
pub async fn raw_create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let mut controllers = Config::try_get()?.controllers.clone();
    controllers.push(canister_self()); // root always controls

    crate::interface::ic::create_canister(controllers, cycles).await
}

//
// PHASE 2: Installation
//

/// Install code + initial state into a new canister.
#[allow(clippy::cast_precision_loss)]
async fn install_canister(
    pid: Principal,
    ty: &CanisterType,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Fetch wasm
    let wasm = WasmRegistry::try_get(ty)?;

    // Canister entry
    let canister_entry = SubnetTopology::try_get(pid)?;

    // Construct initial state
    // the view is the smaller version of the CanisterEntry
    let state = CanisterStateData {
        canister: Some(canister_entry.into()),
    };
    let parents: Vec<CanisterSummary> = SubnetTopology::parents(pid);

    // Install code
    install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (state, parents, extra_arg),
    )
    .await?;

    // Flip to Installed
    SubnetTopology::install(pid, wasm.module_hash())?;

    log!(
        Log::Ok,
        "⚡ install_canister: {pid} ({ty}, {:.2}KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
