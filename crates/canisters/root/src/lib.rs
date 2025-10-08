//!
//! Root demo canister that orchestrates the other sample canisters for tests.
//! Lives in `crates/canisters` purely to showcase cross-canister workflows.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error, canister,
    ops::{
        request::{CreateCanisterParent, create_canister_request},
        response::CreateCanisterResponse,
        root::root_create_canisters,
    },
    prelude::*,
};

//
// ICU
//

canic_start_root!();

async fn canic_setup() {}

async fn canic_install() {
    root_create_canisters().await.unwrap();
}

async fn canic_upgrade() {}

// WASMS
pub static WASMS: &[(CanisterType, &[u8])] = &[
    (
        canister::BLANK,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/blank/blank.wasm.gz"),
    ),
    (
        canister::DELEGATION,
        #[cfg(canic_github_ci)]
        &[],
        #[cfg(not(canic_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/delegation/delegation.wasm.gz"),
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

// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::Caller, None).await
}

// end
export_candid!();
