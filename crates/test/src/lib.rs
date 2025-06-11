use icu::prelude::*;

pub static TEST: &str = "test";

//
// ICU
//

icu_start_root!(TEST);

#[update]
async fn init_async() {
    ::icu::log!(::icu::Log::Warn, "hello from init_async!!");
}

export_candid!();
