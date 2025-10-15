//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    ops::request::{CreateCanisterParent, CreateCanisterResponse, create_canister_request},
    prelude::*,
    types::{Account, TC},
};
use canic_internal::canister;

//
// CANIC
//

canic_start_root!();

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

// get_current_subnet_pid
#[query(composite)]
async fn get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    canic::interface::ic::get_current_subnet_pid().await
}

// convert_icp_to_cycles
#[update]
#[allow(clippy::cast_possible_truncation)]
async fn convert_icp_to_cycles() -> Result<(), Error> {
    let acc = Account::from(msg_caller());
    let cycles = (TC * 2) as u64;

    canic::interface::ic::cycles::convert_icp_to_cycles(acc, cycles).await
}

// get_icp_xdr_conversion_rate
#[query(composite)]
async fn get_icp_xdr_conversion_rate() -> Result<f64, Error> {
    canic::interface::ic::cycles::get_icp_xdr_conversion_rate().await
}

// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::Caller, None).await
}

// end
export_candid!();
