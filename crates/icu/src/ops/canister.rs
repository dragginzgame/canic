use crate::{
    Error,
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    config::Config,
    interface::{ic::install_code, prelude::*},
    memory::{CanisterPool, CanisterStateData, subnet::SubnetRegistry},
    ops::sync::root_cascade,
    state::wasm::WasmRegistry,
};
use candid::Principal;

//
// HIGH-LEVEL FLOW
//

/// Create + install a new canister of given type under parent.
/// Handles allocation, registry updates, wasm install, and root cascade.
pub async fn create_and_install_canister(
    ty: &CanisterType,
    parent: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // Validate type + wasm availability up-front
    Config::try_get_canister(ty)?; // must exist in config
    WasmRegistry::try_get(ty)?; // must have wasm

    // Phase 0: allocate PID + cycles
    let (pid, cycles) = allocate_canister(ty).await?;

    // Phase 1: mark Created in registry
    SubnetRegistry::create(pid, ty, parent);

    // Phase 2: install wasm + flip to Installed
    install_canister(pid, ty, extra_arg).await?;

    log!(Log::Ok, "⚡ create_canister: {pid} ({ty}, {cycles})");

    // Phase 3: cascade updated state from root
    root_cascade().await?;

    Ok(pid)
}

//
// PHASE 0: Allocation
//

/// Allocate a canister ID + cycles, preferring the pool.
/// Returns (pid, cycles).
pub async fn allocate_canister(ty: &CanisterType) -> Result<(Principal, Cycles), Error> {
    // Try pool first
    if let Some((pid, entry)) = CanisterPool::pop_first() {
        log!(Log::Ok, "⚡ reusing {pid} from pool ({entry:?})");
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
    let controllers = get_controllers()?;

    crate::interface::ic::create_canister(controllers, cycles).await
}

/// Get list of controllers: hardcoded from config plus root.
fn get_controllers() -> Result<Vec<Principal>, Error> {
    let mut controllers = Config::try_get()?.controllers.clone();
    controllers.push(canister_self()); // root always controls

    Ok(controllers)
}

//
// PHASE 2: Installation
//

/// Install code + initial state into a new canister.
async fn install_canister(
    pid: Principal,
    ty: &CanisterType,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Fetch wasm
    let wasm = WasmRegistry::try_get(ty)?;

    // Canister entry from registry
    let entry = SubnetRegistry::try_get(pid)?;

    // Construct initial state
    let state = CanisterStateData {
        entry: Some(entry),
        root_pid: Some(canister_self()),
    };

    // Install code
    install_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        (state, extra_arg),
    )
    .await?;

    // Flip registry to Installed
    SubnetRegistry::install(pid, wasm.module_hash())?;

    log!(
        Log::Ok,
        "⚡ install_canister: {pid} ({ty}, {:.2}KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
