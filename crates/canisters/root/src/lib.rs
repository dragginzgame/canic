//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    core::{
        access::policy::is_prime_subnet,
        dto::rpc::{CreateCanisterParent, CreateCanisterResponse},
        ops::{rpc::create_canister_request, runtime::wasm::WasmOps},
    },
    prelude::*,
};
use canic_internal::canister;
use std::collections::HashMap;

//
// CANIC
//

canic::start_root!();

canic::eager_init!({
    // Populate the in-memory WASM registry for provisioning before bootstrap.
    WasmOps::import_static_quiet(WASMS);
});

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

//
// WASMS
//

#[cfg(target_arch = "wasm32")]
const APP_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/app/app.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const APP_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const AUTH_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/auth/auth.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const AUTH_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const BLANK_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/blank/blank.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const BLANK_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SCALE_HUB_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/scale_hub/scale_hub.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SCALE_HUB_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SCALE_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/scale/scale.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SCALE_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SHARD_HUB_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/shard_hub/shard_hub.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SHARD_HUB_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SHARD_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/shard/shard.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SHARD_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const TEST_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const TEST_WASM: &[u8] = &[];

pub static WASMS: &[(CanisterRole, &[u8])] = &[
    (canister::APP, APP_WASM),
    (canister::AUTH, AUTH_WASM),
    (canister::BLANK, BLANK_WASM),
    (canister::SCALE_HUB, SCALE_HUB_WASM),
    (canister::SCALE, SCALE_WASM),
    (canister::SHARD_HUB, SHARD_HUB_WASM),
    (canister::SHARD, SHARD_WASM),
    (canister::TEST, TEST_WASM),
];

/// create_blank
/// Controller-only helper for local Canic testing.
#[canic_update(guard(app), auth_any(is_controller), policy(is_prime_subnet))]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::ThisCanister, None).await
}

/// stress_perf
/// Synthetic CPU-heavy endpoint to validate perf instrumentation.
#[canic_update(guard(app), auth_any(is_controller))]
async fn stress_perf(rounds: u32) -> Result<u64, Error> {
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

    Ok(acc)
}

// end
export_candid!();
