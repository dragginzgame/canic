use icu::{
    interface::root::root_create_canisters,
    prelude::*,
    state::{CanisterAttributes, CanisterRegistry},
};

//
// ICU
//

icu_start_root!("root");

#[update]
async fn init_async() {
    icu_config!("../../icu.toml");

    register_canisters();
    root_create_canisters().await.unwrap();

    // let config = icu::config::Config::get();
}

// register_canisters
fn register_canisters() {
    let canisters: &[(&'static str, CanisterAttributes, &'static [u8])] = &[(
        "test",
        CanisterAttributes {
            auto_create: true,
            indexable: true,
        },
        &[],
        //include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    )];

    for (path, atts, wasm) in canisters {
        CanisterRegistry::insert(path, atts, wasm).unwrap();
    }
}

// end
export_candid!();
