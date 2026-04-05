//!
//! Root demo canister that orchestrates the sample canisters in the reference
//! topology.
//!

#![allow(clippy::unused_async)]

use canic::api::auth::DelegationApi;

//
// CANIC
//

canic::start_root!();

// Warm root auth key material outside the first live delegation request path.
async fn canic_setup() {
    let _ = DelegationApi::prewarm_root_key_material().await;
}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::cdk::export_candid_debug!();
