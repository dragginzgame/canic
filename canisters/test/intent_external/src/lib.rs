//!
//! Minimal external canister used to force an inter-canister await.
//!

#![expect(clippy::unused_async)]

use ic_cdk::update;

#[update]
async fn perform() {
    ic_cdk::println!("intent_external: perform");
}
