//! Module: caller_authority
//!
//! Disposable receiver probes for local caller, target and post-await authority.
//!
//! Counters use the fixture's existing application storage; publication remains Root-owned.

use crate::protocol;
use canic::{Error, access::caller_authority, prelude::*};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_platform::{
    candid::contract::{DecoderConfig, decode_args_with_config, encode_args},
    cdk::call::Call,
};
use std::cell::Cell;

const EFFECT_ROW: u64 = 999;

thread_local! {
    static HELD: Cell<bool> = const { Cell::new(false) };
    static WAITING: Cell<bool> = const { Cell::new(false) };
}

#[update(public, wire(id = protocol::CALLER_PROBE, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn test_caller_probe() -> Result<ic_platform::Principal, Error> {
    caller_authority::admit("test_notify")
        .map(|ticket| ticket.caller().into())
        .map_err(Into::into)
}

#[update(requires(caller::is_controller()), wire(id = protocol::CALLER_TARGET, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn test_caller_target(target: ic_platform::Principal) -> Result<ic_platform::Principal, Error> {
    caller_authority::select_target(target.into(), "test_metrics_target")
        .map(|ticket| ticket.target().into())
        .map_err(Into::into)
}

#[update(requires(caller::is_controller()), wire(id = protocol::CALLER_HOLD, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn test_caller_hold(held: bool) -> Result<(), Error> {
    HELD.set(held);
    Ok(())
}

#[query(requires(caller::is_controller()), wire(id = protocol::CALLER_EFFECT_STATUS, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn test_caller_effect_status() -> Result<(bool, u64), Error> {
    Ok((WAITING.get(), effects()))
}

#[update(public, wire(id = protocol::CALLER_EFFECT, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_caller_effect() -> Result<u64, Error> {
    let ticket = caller_authority::admit("test_notify")?;
    WAITING.set(true);
    for _ in 0..256 {
        let response = Call::unbounded_wait(Principal::management_canister().into(), "raw_rand")
            .take_raw_args(encode_args(()).expect("fixture consensus arguments"))
            .await
            .expect("fixture consensus boundary");
        let _: (Vec<u8>,) = decode_args_with_config(&response.into_bytes(), &DecoderConfig::new())
            .expect("fixture randomness reply");
        if !HELD.get() {
            WAITING.set(false);
            caller_authority::revalidate(&ticket)?;
            let next = effects().checked_add(1).expect("fixture counter bound");
            crate::reinstall_fixture::insert(EFFECT_ROW, next);
            return Ok(next);
        }
    }
    ic_platform::cdk::api::trap("caller-effect fixture barrier exhausted");
}

fn effects() -> u64 {
    crate::reinstall_fixture::rows()
        .into_iter()
        .find(|row| row.id == EFFECT_ROW)
        .map_or(0, |row| row.value)
}
