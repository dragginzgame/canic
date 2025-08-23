use crate::{
    Error, Log,
    interface::{
        InterfaceError,
        ic::{create_canister, get_cycles, uninstall_code},
    },
    log,
    memory::{CanisterPool, CanisterRegistry, CanisterState},
    types::{Cycles, TC},
};
use candid::Principal;

///
/// Constants
///

const POOL_CANISTER_CYCLES: Cycles = Cycles::new(5 * TC);

///
/// create_pool_canister
/// creates an empty canister and registers it with the CanisterPool
///
pub async fn create_pool_canister() -> Result<Principal, Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    let canister_pid = create_canister(POOL_CANISTER_CYCLES).await?;

    log!(
        Log::Ok,
        "💧 create_pool_canister: {canister_pid} ({POOL_CANISTER_CYCLES})",
    );

    CanisterPool::register(canister_pid, POOL_CANISTER_CYCLES);

    Ok(canister_pid)
}

///
/// move_canister_to_pool
///
pub async fn move_canister_to_pool(canister_pid: Principal) -> Result<(), Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    // uninstall code
    uninstall_code(canister_pid).await?;

    // remove from registry
    let canister_type = if let Some(entry) = CanisterRegistry::remove(&canister_pid) {
        entry.canister_type.to_string()
    } else {
        String::from("unregistered")
    };

    // register to Pool
    let cycles = get_cycles(canister_pid).await?;
    CanisterPool::register(canister_pid, cycles);

    log!(
        Log::Ok,
        "💧 move_canister_to_pool: {canister_pid} (was {canister_type}) ({cycles})",
    );

    Ok(())
}
