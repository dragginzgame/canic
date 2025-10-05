//! Lifecycle helpers for the shared reserve pool.
//!
//! The root canister maintains an inventory of empty canisters that can be
//! handed out quickly when scaling. These helpers create new reserve
//! canisters, top them up with cycles, and reclaim existing canisters into the
//! pool.

use crate::{
    Error, Log,
    interface::ic::get_cycles,
    memory::root::CanisterReserve,
    ops::{
        canister::{raw_create_canister, uninstall_and_delete_canister},
        prelude::*,
    },
    types::{Cycles, TC},
};

/// Default cycle balance for freshly created reserve canisters (5 T cycles).
const RESERVE_CANISTER_CYCLES: u128 = 5 * TC;

/// Create an empty reserve canister controlled by root.
pub async fn create_reserve_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(RESERVE_CANISTER_CYCLES);
    let canister_pid = raw_create_canister(cycles.clone()).await?;

    log!(Log::Ok, "🪶  create_reserve: {canister_pid} ({cycles})",);

    CanisterReserve::register(canister_pid, cycles);

    Ok(canister_pid)
}

/// Move an existing canister into the reserve pool after uninstalling it.
pub async fn move_canister_to_reserve(canister_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    // uninstall and delete
    let canister_type = uninstall_and_delete_canister(canister_pid).await?;

    // register to Reserve
    let cycles = get_cycles(canister_pid).await?;
    CanisterReserve::register(canister_pid, cycles.clone());

    log!(
        Log::Ok,
        "🪶  move_canister_to_reserve: {canister_pid} (was {canister_type}) ({cycles})",
    );

    Ok(())
}
