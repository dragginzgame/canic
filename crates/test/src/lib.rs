use candid::Principal;
use icu::prelude::*;

pub static TEST: &str = "test";

//
// ICU
//

icu_start!(TEST, extra = Principal);

fn _init(owner_id: Principal) {
    log!(Log::Warn, "{owner_id:?}");
}

#[expect(clippy::unused_async)]
async fn _init_async() {}
