use icu::prelude::*;

//
// ICU
//

icu_start!("test");

#[update]
#[allow(clippy::unused_async)]
async fn init_async(args: Option<Vec<u8>>) {
    log!(Log::Warn, "hello from init_async!! args: {args:?}");
}

export_candid!();
