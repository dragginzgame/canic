use icu::{
    Error,
    canister::{Attributes, CanisterRegistry, IndexingPolicy},
    interface::{request::create_canister_request, root::root_create_canisters},
    prelude::*,
};

//
// ICU
//

icu_start_root!("root");

async fn icu_init() {
    register_canisters();
    root_create_canisters().await.unwrap();

    // let config = icu::config::Config::get();
}

async fn icu_startup() {
    icu_config!("../../icu.toml");
}

// register_canisters
fn register_canisters() {
    let canisters: &[(&'static str, Attributes, &'static [u8])] = &[(
        "test",
        Attributes {
            auto_create: Some(2),
            indexing: IndexingPolicy::Limited(2),
        },
        &[],
        //include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    )];

    for (path, atts, wasm) in canisters {
        CanisterRegistry::insert(path, atts, wasm);
    }
}

// create_test
#[update]
async fn create_test() -> Result<Principal, Error> {
    create_canister_request::<()>("test", None).await
}

// end
export_candid!();
