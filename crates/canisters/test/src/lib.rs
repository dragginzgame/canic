use icu::prelude::*;

//
// ICU
//

icu_start!("test");

#[update]
async fn init_async(args: Option<Vec<u8>>) {
    ::icu::log!(::icu::Log::Warn, "hello from init_async!! args: {args:?}");
}

export_candid!();
