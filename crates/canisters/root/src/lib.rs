use icu::{
    interface::state::root::canister_registry::{self, CanisterDef},
    prelude::*,
};

pub const ROOT: &str = "root";

//
// ICU
//

icu_start_root!(ROOT);

#[update]
async fn init_async() {
    register_canisters();
}

// register_canisters
fn register_canisters() {
    let canisters: &[(&'static str, CanisterDef, &'static [u8])] = &[(
        "test",
        CanisterDef {
            auto_create: true,
            is_sharded: false,
        },
        include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz"),
    )];

    for (path, def, wasm) in canisters {
        canister_registry::add_canister(path, def, wasm).unwrap();
    }
}

export_candid!();
