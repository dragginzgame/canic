#![allow(clippy::unused_async)]

use candid::Principal;
use icu::{
    Error,
    canister::{EXAMPLE, PLAYER, PLAYER_HUB},
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
        EXAMPLE,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/example/example.wasm.gz"),
    ),
    (
        PLAYER,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/player/player.wasm.gz"),
    ),
    (
        PLAYER_HUB,
        #[cfg(icu_github_ci)]
        &[],
        #[cfg(not(icu_github_ci))]
        include_bytes!("../../../../.dfx/local/canisters/player_hub/player_hub.wasm.gz"),
    ),
];

///
/// ENDPOINTS
///

// create_example (demo)
#[update]
async fn create_example() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&EXAMPLE, None).await
}

// create_player (demo)
#[update]
async fn create_player() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&PLAYER, None).await
}

// create_player_hub (demo)
#[update]
async fn create_player_hub() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&PLAYER_HUB, None).await
}

#[update]
async fn get_icp_rate() -> Result<f64, Error> {
    icu::interface::ic::get_icp_xdr_conversion_rate().await
}

// end
export_candid!();
