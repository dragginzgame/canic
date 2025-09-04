// Example: composing and referencing auth rules.
// This example compiles without the `ic` feature and does not execute rules.

use icu::types::Principal;

fn main() {
    // Build a rule future (not awaited here).
    let _fut = icu::auth::is_same_canister(Principal::anonymous());
    println!("auth rules example compiled");
}
