#![allow(clippy::unused_async)]

use icu::{
    Error, TEST,
    canister::CanisterType,
    interface::{
        ic::create_canister_pool, request::create_canister_request,
        response::CreateCanisterResponse, root::root_create_canisters,
    },
    prelude::*,
    state::canister::{Attributes, Canister, DirectoryPolicy},
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

// CANISTERS
pub static CANISTERS: &[(&CanisterType, Canister)] = &[(
    TEST,
    Canister {
        attributes: Attributes {
            auto_create: Some(2),
            directory: DirectoryPolicy::Limited(6),
        },
        #[cfg(icu_github_ci)]
        wasm: &[],
        #[cfg(not(icu_github_ci))]
        wasm: include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    },
)];

///
/// ENDPOINTS
///

// create_test
#[update]
async fn create_test() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(TEST, None).await
}

// create_pool
#[update]
async fn create_pool() -> Result<Principal, Error> {
    create_canister_pool().await
}

// end
export_candid!();
