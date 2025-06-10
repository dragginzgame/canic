use candid::Principal;
use icu::prelude::*;

pub static TEST: &str = "test";

//
// ICU
//

icu_start!(TEST);

fn _init() {}

async fn _init_async(owner_id: Principal) {
    log!(Log::Warn, "{owner_id:?}");
}
