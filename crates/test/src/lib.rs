use candid::Principal;
use icu::prelude::*;

pub static TEST: &str = "test";

//
// ICU
//

icu_start!(TEST);

async fn _init(owner_id: Principal) {
    log!(Log::Warn, "{owner_id:?}");
}
