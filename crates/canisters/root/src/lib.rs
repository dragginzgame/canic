//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    ops::request::{CreateCanisterParent, CreateCanisterResponse, create_canister_request},
    prelude::*,
};
use canic_internal::canister;

//
// CANIC
//

canic::start_root!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

// WASMS
pub static WASMS: &[(CanisterType, &[u8])] = &[
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
];

///
/// ENDPOINTS
///

// convert_icp_to_cycles
/*
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
// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::Caller, None).await
}

/// test_perf
/// just checks to see if the perf macros compile
#[ic_cdk::update]
async fn test_perf() {
    // Start the cumulative measurement for the call
    perf_start!();

    //
    // First workload
    //
    perf!("starting workload 1");
    let mut acc1 = 0u64;
    for i in 0..50_000 {
        acc1 = acc1.wrapping_add(i);
    }

    perf!("end");
}

// end
export_candid!();
