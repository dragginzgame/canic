//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    core::ops::request::{CreateCanisterParent, CreateCanisterResponse, create_canister_request},
    prelude::*,
};
use canic_internal::canister;
use std::collections::HashMap;

//
// CANIC
//

canic::start_root!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

// WASMS
pub static WASMS: &[(CanisterRole, &[u8])] = &[
    (
        canister::APP,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/app/app.wasm.gz"),
    ),
    (
        canister::AUTH,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/auth/auth.wasm.gz"),
    ),
    (
        canister::BLANK,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/blank/blank.wasm.gz"),
    ),
    (
        canister::SCALE_HUB,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/scale_hub/scale_hub.wasm.gz"),
    ),
    (
        canister::SCALE,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/scale/scale.wasm.gz"),
    ),
    (
        canister::SHARD_HUB,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/shard_hub/shard_hub.wasm.gz"),
    ),
    (
        canister::SHARD,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/shard/shard.wasm.gz"),
    ),
    (
        canister::TEST,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    ),
];

/*
// convert_icp_to_cycles
#[update]
#[allow(clippy::cast_possible_truncation)]
async fn convert_icp_to_cycles() -> Result<(), Error> {
    canic::ops::ext::cycles::CycleTrackerOps::convert_caller_icp_to_cycles((TC * 2) as u64).await
}

// get_icp_xdr_conversion_rate
#[query(composite)]
async fn get_icp_xdr_conversion_rate() -> Result<f64, Error> {
    canic::interface::ic::cycles::get_icp_xdr_conversion_rate().await
}
*/

/// create_blank
/// no authentication needed as its for local canic testing
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    perf_scope!("test");

    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::ThisCanister, None).await
}

/// stress_perf
/// Synthetic CPU-heavy endpoint to validate perf instrumentation.
#[update]
async fn stress_perf(rounds: u32) -> Result<u64, Error> {
    // Measure total cost of the endpoint
    perf_scope!("endpoint:stress_perf");

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

    perf!("a");

    // Phase 2: repeated traversal + mutation
    for _ in 0..4 {
        for (k, v) in &mut map {
            *v = v.wrapping_add(*k).rotate_right((*k & 31) as u32) ^ acc;

            acc = acc.wrapping_add(*v);
        }
    }

    perf!("b");

    // Phase 3: reduction pass
    for (k, v) in map {
        acc ^= k.wrapping_mul(v.rotate_left(7));
    }

    perf!("c");

    // Phase 4: allocate memory
    let mut v = Vec::new();
    for i in 0..rounds {
        v.push(i);
    }
    let _ = v;

    Ok(acc)
}

// end
export_candid!();
