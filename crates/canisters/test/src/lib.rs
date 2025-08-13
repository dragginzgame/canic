use icu::{Error, interface::request::create_canister_request, prelude::*};

//
// ICU
//

icu_start!("test");

#[allow(clippy::unused_async)]
async fn icu_init(args: Option<Vec<u8>>) {
    log!(Log::Warn, "init_async: args = {args:?}");
}

async fn icu_startup() {
    icu_config!("../../icu.toml");
}

// create_test
#[update]
async fn create_test() -> Result<Principal, Error> {
    create_canister_request::<()>("test", None).await
}

export_candid!();
