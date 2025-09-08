#![allow(clippy::unused_async)]

use candid::Principal;
use icu::{
    Error, canister,
    ops::{
        request::create_canister_request, response::CreateCanisterResponse,
        root::root_create_canisters,
    },
    prelude::*,
};

//
// ICU
//

icu_start_root!();

const fn icu_setup() {}

async fn icu_install() {
    root_create_canisters().await.unwrap();
}

async fn icu_upgrade() {}

// WASMS
pub static WASMS: &[(CanisterType, &[u8])] = &[
    (
        canister::EXAMPLE,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/example/example.wasm.gz"),
    ),
    (
        canister::PLAYER_HUB,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/player_hub/player_hub.wasm.gz"),
    ),
    (
        canister::GAME,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/game/game.wasm.gz"),
    ),
    (
        canister::INSTANCE,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/instance/instance.wasm.gz"),
    ),
];

///
/// ENDPOINTS
///

// create_example (demo)
#[update]
async fn create_example() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&canister::EXAMPLE, None).await
}

#[update]
async fn get_icp_rate() -> Result<f64, Error> {
    icu::interface::ic::get_icp_xdr_conversion_rate().await
}

// end
export_candid!();
