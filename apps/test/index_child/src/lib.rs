#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, prelude::*};
use std::cell::Cell;

thread_local! {
    static STARTUP_CALLS: Cell<u32> = const { Cell::new(0) };
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(args: Option<Vec<u8>>) {
    let rounds = args.map_or(4, |bytes| {
        candid::decode_one::<u32>(&bytes).expect("fixture startup rounds")
    });
    assert!(rounds <= 256, "bounded fixture startup work");
    for _ in 0..rounds {
        Call::bounded_wait(Principal::management_canister(), "raw_rand")
            .with_args(())
            .expect("fixture consensus arguments")
            .execute_candid::<Vec<u8>>()
            .await
            .expect("fixture startup consensus boundary");
        STARTUP_CALLS.set(STARTUP_CALLS.get() + 1);
    }
}
async fn canic_upgrade() {}

/// Observe real asynchronous startup work after application admission opens.
#[canic_query(requires(caller::is_controller()))]
fn test_startup_calls() -> Result<u32, Error> {
    Ok(STARTUP_CALLS.get())
}

/// Prove direct Fleet-admitted ingress for an index-created managed child.
#[canic_query(requires(caller::is_fleet_admitted()))]
async fn test_fleet_admission_probe() -> Result<Principal, Error> {
    Ok(ic_cdk::api::msg_caller())
}

canic::finish!();
