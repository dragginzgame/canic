//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

#[cfg(debug_assertions)]
use canic::{
    Error, api::rpc::RpcApi, dto::rpc::CreateCanisterParent, dto::rpc::CreateCanisterResponse,
};
use canic::{api::canister::template::WasmStoreBootstrapApi, prelude::*};
#[cfg(debug_assertions)]
use canic_internal::canister;
#[cfg(debug_assertions)]
use std::collections::HashMap;

include!(concat!(
    env!("OUT_DIR"),
    "/embedded_store_release_catalog.rs"
));

//
// CANIC
//

canic::start_root!(
    init = {
        WasmStoreBootstrapApi::import_embedded_release_catalog(
            embedded_wasm_store_release_catalog(),
        );
    }
);

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[cfg(debug_assertions)]
/// create_minimal
/// Controller-only helper for local Canic testing.
#[canic_update(requires(caller::is_controller()))]
async fn create_minimal() -> Result<CreateCanisterResponse, Error> {
    RpcApi::create_canister_request::<()>(
        &canister::MINIMAL,
        CreateCanisterParent::ThisCanister,
        None,
    )
    .await
}

#[cfg(debug_assertions)]
/// stress_perf
/// Synthetic CPU-heavy endpoint to validate perf instrumentation.
#[canic_update(requires(caller::is_controller()))]
async fn stress_perf(rounds: u32) -> Result<u64, Error> {
    Ok(stress_perf_compute(rounds))
}

#[cfg(debug_assertions)]
fn stress_perf_compute(rounds: u32) -> u64 {
    let mut acc: u64 = 0;
    let mut map: HashMap<u64, u64> = HashMap::with_capacity(rounds as usize);

    // Phase 1: populate + heavy arithmetic
    for i in 0..rounds {
        let mut x = u64::from(i).wrapping_mul(0x9E37_79B9_7F4A_7C15);

        // Inner arithmetic loop (hot path)
        for j in 0..32 {
            x = x.wrapping_add(j).rotate_left((j % 63) as u32) ^ 0xA5A5_A5A5_A5A5_A5A5;
        }

        map.insert(u64::from(i), x);
        acc = acc.wrapping_add(x);
    }

    // Phase 2: repeated traversal + mutation
    for _ in 0..4 {
        for (k, v) in &mut map {
            *v = v.wrapping_add(*k).rotate_right((*k & 31) as u32) ^ acc;

            acc = acc.wrapping_add(*v);
        }
    }

    // Phase 3: reduction pass
    for (k, v) in map {
        acc ^= k.wrapping_mul(v.rotate_left(7));
    }

    // Phase 4: allocate memory
    let mut v = Vec::new();
    for i in 0..rounds {
        v.push(i);
    }
    let _ = v;

    acc
}

canic::export_candid!();
