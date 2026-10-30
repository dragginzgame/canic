#![expect(clippy::unused_async)]
mod protocol;

use canic::{Error, prelude::*};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_platform::{
    candid::contract::{DecoderConfig, decode_args_with_config, encode_args},
    cdk::call::Call,
};
use std::cell::Cell;

thread_local! {
    static STARTUP_CALLS: Cell<u32> = const { Cell::new(0) };
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(args: Option<Vec<u8>>) {
    let rounds = args.map_or(4, |bytes| {
        canic::__internal::contracts::protocol::initialization::decode::<u32>(&bytes)
            .expect("fixture startup rounds")
    });
    assert!(rounds <= 256, "bounded fixture startup work");
    for _ in 0..rounds {
        let response = Call::bounded_wait(Principal::management_canister().into(), "raw_rand")
            .take_raw_args(encode_args(()).expect("fixture consensus arguments"))
            .await
            .expect("fixture startup consensus boundary");
        let _: (Vec<u8>,) = decode_args_with_config(&response.into_bytes(), &DecoderConfig::new())
            .expect("fixture randomness reply");
        STARTUP_CALLS.set(STARTUP_CALLS.get() + 1);
    }
}
async fn canic_upgrade() {}

/// Observe real asynchronous startup work after application admission opens.
#[query(requires(caller::is_controller()), wire(id = protocol::STARTUP_CALLS, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn test_startup_calls() -> Result<u32, Error> {
    Ok(STARTUP_CALLS.get())
}

/// Prove direct Fleet-admitted ingress for an index-created managed child.
#[query(requires(caller::is_fleet_admitted()), wire(id = protocol::ADMISSION, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_fleet_admission_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

canic::finish!();
