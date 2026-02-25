//!
//! Minimal authority canister for intent reservation tests.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic_core::api::ic::call::{Call, IntentKey, IntentReservation};
use ic_cdk::update;
use std::cell::RefCell;

const CAPACITY: u64 = 1;
const CANIC_MEMORY_MIN: u8 = canic_core::CANIC_MEMORY_MIN;
const CANIC_MEMORY_MAX: u8 = canic_core::CANIC_MEMORY_MAX;

thread_local! {
    static EXTERNAL: RefCell<Option<Principal>> = const { RefCell::new(None) };
}

#[ic_cdk::init]
fn init(external: Principal) {
    init_memory();
    ic_cdk::println!("intent_authority: init external={external}");
    EXTERNAL.with(|cell| *cell.borrow_mut() = Some(external));
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    init_memory();
    ic_cdk::println!("intent_authority: post_upgrade memory initialized");
}

#[update]
async fn buy(qty: u64) -> Result<(), String> {
    // Idempotent bootstrap guard for custom test canister wiring.
    init_memory();
    ic_cdk::println!("intent_authority: buy start qty={qty}");

    let external = external_principal()?;
    ic_cdk::println!("intent_authority: call external perform {}", external);
    let intent = IntentReservation::new(intent_key()?, qty).with_max_in_flight(CAPACITY);
    let call_builder = Call::unbounded_wait(external, "perform")
        .with_intent(intent)
        .with_arg(())
        .map_err(|err| err.to_string())?;
    let call_result = call_builder.execute().await;

    match call_result {
        Ok(_) => {
            ic_cdk::println!("intent_authority: external ok");
            Ok(())
        }
        Err(call_err) => {
            ic_cdk::println!("intent_authority: external failed err={call_err}");
            Err(format!("external call failed: {call_err}"))
        }
    }
}

fn init_memory() {
    canic_core::memory::runtime::init_eager_tls();
    canic_core::memory::runtime::registry::MemoryRegistryRuntime::init(Some((
        canic_core::CRATE_NAME,
        CANIC_MEMORY_MIN,
        CANIC_MEMORY_MAX,
    )))
    .expect("memory registry init should succeed");
}

fn intent_key() -> Result<IntentKey, String> {
    IntentKey::try_new("capacity").map_err(|err| err.to_string())
}

fn external_principal() -> Result<Principal, String> {
    EXTERNAL
        .with(|cell| *cell.borrow())
        .ok_or_else(|| "external canister not initialized".to_string())
}
