use icu::{
    Error,
    canister::{Attributes, Canister, CanisterRegistry, IndexingPolicy},
    interface::request::create_canister_request,
    prelude::*,
};

//
// ICU
//

icu_start_root!();

async fn icu_init() {}

async fn icu_startup() {
    icu_config!("../../icu.toml");

    CanisterRegistry::import(CANISTERS);
}

// CANISTERS
pub const CANISTERS: &[Canister] = &[(Canister {
    kind: "test",
    attributes: Attributes {
        auto_create: Some(2),
        indexing: IndexingPolicy::Limited(2),
    },
    //wasm: include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    wasm: &[],
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
