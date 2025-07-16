use icu::{
    interface::state::create_canisters,
    prelude::*,
    state::{CanisterAttributes, CanisterRegistry},
};

//
// ICU
//

icu_start_root!("root");

#[update]
async fn init_async() {
    register_canisters();
    create_canisters(&[]).await.unwrap()
}

// register_canisters
fn register_canisters() {
    let canisters: &[(&'static str, CanisterAttributes, &'static [u8])] = &[(
        "test",
        CanisterAttributes {
            auto_create: true,
            is_sharded: false,
        },
        include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    )];

    for (path, def, wasm) in canisters {
        CanisterRegistry::add_canister(path, def, wasm).unwrap();
    }
}

export_candid!();
