#![allow(clippy::unused_async)]

use icu::{
    Error, TEST,
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
pub static WASMS: &[(CanisterType, &[u8])] = &[(
    TEST,
    #[cfg(icu_github_ci)]
    &[],
    #[cfg(not(icu_github_ci))]
    include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
)];

///
/// ENDPOINTS
///

// create_test
#[update]
async fn create_test() -> Result<CreateCanisterResponse, Error> {
    create_canister_request::<()>(&TEST, None).await
}

#[update]
async fn get_icp_rate() -> Result<f64, Error> {
    icu::interface::ic::get_icp_xdr_conversion_rate().await
}

// end
export_candid!();
