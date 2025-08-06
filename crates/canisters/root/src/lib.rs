use icu::{interface::state::create_canisters, prelude::*};

//
// ICU
//

icu_start_root!("root");

#[update]
async fn init_async() {
    icu_config!("../../icu.toml");

    register_canisters();
    create_canisters(&[]).await.unwrap();

    // let config = icu::config::Config::get();
}

// register_canisters
fn register_canisters() {
    // let canisters: &[(&'static str, CanisterAttributes, &'static [u8])] = &[(
    //     "test",
    //     CanisterAttributes {
    //         auto_create: true,
    //         is_sharded: false,
    //     },
    //     include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    // )];

    // for (path, def, wasm) in canisters {
    //     CanisterRegistry::insert(path, def, wasm).unwrap();
    // }
}

export_candid!();
