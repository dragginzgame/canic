use icu::{Error, interface::request::create_canister_request, prelude::*};

//
// ICU
//

icu_start!("test");

#[update]
#[allow(clippy::unused_async)]
async fn init_async(args: Option<Vec<u8>>) {
    log!(Log::Warn, "init_async: args = {args:?}");
}

// create_test
#[update]
async fn create_test() -> Result<Principal, Error> {
    create_canister_request::<()>("test", None).await
}

export_candid!();
