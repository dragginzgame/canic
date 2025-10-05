// Example: composing and referencing auth rules.
// This example compiles on the host without executing IC calls.

use canic::auth;
use canic::types::Principal;

fn main() {
    // Build a rule future (not awaited here).
    let _fut = auth::is_same_canister(Principal::anonymous());

    println!("auth rules example compiled");
}
