#![allow(clippy::unused_async)]

use candid::Principal;
use icu::{
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

icu_start_root!();

async fn icu_setup() {}

async fn icu_install() {
    root_create_canisters().await.unwrap();
}

async fn icu_upgrade() {}

// WASMS
pub static WASMS: &[(CanisterType, &[u8])] = &[
    (
        canister::BLANK,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/blank/blank.wasm.gz"),
    ),
    (
        canister::DELEGATION,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/delegation/delegation.wasm.gz"),
    ),
    (
        canister::SCALE_HUB,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/scale_hub/scale_hub.wasm.gz"),
    ),
    (
        canister::SCALE,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/scale/scale.wasm.gz"),
    ),
    (
        canister::SHARD_HUB,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/shard_hub/shard_hub.wasm.gz"),
    ),
    (
        canister::SHARD,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/shard/shard.wasm.gz"),
    ),
];

///
/// ENDPOINTS
///

// create_blank
#[update]
async fn create_blank() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::Caller, None).await
}

// end
export_candid!();
