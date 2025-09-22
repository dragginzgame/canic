use crate::{
    Error, Log,
    interface::ic::{get_cycles, uninstall_code},
    memory::{CanisterPool, subnet::SubnetRegistry},
    ops::{canister::raw_create_canister, prelude::*},
    types::{Cycles, TC},
};

///
/// Constants
///

const POOL_CANISTER_CYCLES: u128 = 5 * TC;

///
/// create_pool_canister
/// creates an empty canister and registers it with the CanisterPool
///
pub async fn create_pool_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let canister_pid = raw_create_canister(cycles.clone()).await?;

    log!(Log::Ok, "💧 create_pool: {canister_pid} ({cycles})",);

    CanisterPool::register(canister_pid, cycles);

    Ok(canister_pid)
}

///
/// move_canister_to_pool
///
pub async fn move_canister_to_pool(canister_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    // uninstall code
    uninstall_code(canister_pid).await?;

    // remove from registry
    let canister_type = if let Some(canister) = SubnetRegistry::remove(&canister_pid) {
        canister.ty.to_string()
    } else {
        String::from("unregistered")
    };

    // register to Pool
    let cycles = get_cycles(canister_pid).await?;
    CanisterPool::register(canister_pid, cycles.clone());

    log!(
        Log::Ok,
        "💧 move_canister_to_pool: {canister_pid} (was {canister_type}) ({cycles})",
    );

    Ok(())
}
