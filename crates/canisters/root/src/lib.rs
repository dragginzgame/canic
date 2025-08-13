use icu::{
    Error,
    canister::{Attributes, Canister, IndexingPolicy},
    interface::{request::create_canister_request, root::root_create_canisters},
    prelude::*,
};

//
// ICU
//

icu_start_root!();

async fn icu_setup() {
    icu_config!("../../icu.toml");
}

async fn icu_install() {
    root_create_canisters().await.unwrap();
}

async fn icu_upgrade() {}

// CANISTERS
pub const CANISTERS: &[Canister] = &[(Canister {
    kind: "test",
    attributes: Attributes {
        auto_create: Some(2),
        indexing: IndexingPolicy::Limited(2),
    },
    wasm: &[],
    //    wasm: include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
})];

///
/// ENDPOINTS
///

// create_test
#[update]
async fn create_test() -> Result<Principal, Error> {
    create_canister_request::<()>("test", None).await
}

// end
export_candid!();
