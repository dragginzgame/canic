use crate::{
    Error, Log,
    interface::{
        InterfaceError,
        ic::{CreateCanisterResult, create_canister, get_cycles, uninstall_canister},
    },
    log,
    memory::{CanisterPool, CanisterRegistry, CanisterState},
    utils::cycles::format_cycles,
};
use candid::Principal;

///
/// create_pool_canister
/// creates an empty canister and registers it with the CanisterPool
///
pub async fn create_pool_canister() -> Result<Principal, Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    let CreateCanisterResult {
        canister_pid,
        cycles,
        ..
    } = create_canister().await?;

    log!(
        Log::Ok,
        "⚡ create_pool_canister: pid {} ({})",
        canister_pid,
        format_cycles(cycles)
    );

    CanisterPool::register(canister_pid, cycles);

    Ok(canister_pid)
}

///
/// move_canister_to_pool
///
pub async fn move_canister_to_pool(canister_pid: Principal) -> Result<(), Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    // uninstall
    uninstall_canister(canister_pid).await?;

    // remove from registry
    let canister_type = if let Some(entry) = CanisterRegistry::remove(&canister_pid) {
        entry.canister_type.to_string()
    } else {
        log!(
            Log::Warn,
            "⚡ missing canister registry entry for {canister_pid}"
        );
        String::from("[MISSING]")
    };

    // register to Pool
    let cycles = get_cycles(canister_pid).await?;
    CanisterPool::register(canister_pid, cycles);

    log!(
        Log::Ok,
        "⚡ move_canister_to_pool: pid {} (was {}) ({})",
        canister_pid,
        canister_type,
        format_cycles(cycles)
    );

    Ok(())
}
