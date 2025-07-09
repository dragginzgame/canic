use icu::prelude::*;

pub const TEST: &str = "test";

//
// ICU
//

icu_start!(TEST);

#[update]
async fn init_async(args: Option<Vec<u8>>) {
    ::icu::log!(::icu::Log::Warn, "hello from init_async!! args: {args:?}");
}

export_candid!();
