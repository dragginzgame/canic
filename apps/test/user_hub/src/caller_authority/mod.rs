//! Module: caller_authority
//!
//! Disposable receiver probes for local caller, target and post-await authority.
//!
//! Counters use the fixture's existing application storage; publication remains Root-owned.

use candid::Principal;
use canic::{Error, access::caller_authority, prelude::*};
use std::cell::Cell;

const EFFECT_ROW: u64 = 999;

thread_local! {
    static HELD: Cell<bool> = const { Cell::new(false) };
    static WAITING: Cell<bool> = const { Cell::new(false) };
}

#[canic_update(public)]
fn test_caller_probe() -> Result<Principal, Error> {
    caller_authority::admit("test_notify")
        .map(|ticket| ticket.caller())
        .map_err(Into::into)
}

#[canic_update(requires(caller::is_controller()))]
fn test_caller_target(target: Principal) -> Result<Principal, Error> {
    caller_authority::select_target(target, "test_metrics_target")
        .map(|ticket| ticket.target())
        .map_err(Into::into)
}

#[canic_update(requires(caller::is_controller()))]
fn test_caller_hold(held: bool) -> Result<(), Error> {
    HELD.set(held);
    Ok(())
}

#[canic_query(requires(caller::is_controller()))]
fn test_caller_effect_status() -> Result<(bool, u64), Error> {
    Ok((WAITING.get(), effects()))
}

#[canic_update(public)]
async fn test_caller_effect() -> Result<u64, Error> {
    let ticket = caller_authority::admit("test_notify")?;
    WAITING.set(true);
    for _ in 0..256 {
        Call::unbounded_wait(Principal::management_canister(), "raw_rand")
            .with_args(())
            .expect("fixture consensus arguments")
            .execute_candid::<Vec<u8>>()
            .await
            .expect("fixture consensus boundary");
        if !HELD.get() {
            WAITING.set(false);
            caller_authority::revalidate(&ticket)?;
            let next = effects().checked_add(1).expect("fixture counter bound");
            crate::reinstall_fixture::insert(EFFECT_ROW, next);
            return Ok(next);
        }
    }
    ic_cdk::trap("caller-effect fixture barrier exhausted");
}

fn effects() -> u64 {
    crate::reinstall_fixture::rows()
        .into_iter()
        .find(|row| row.id == EFFECT_ROW)
        .map_or(0, |row| row.value)
}
