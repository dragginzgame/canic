//!
//! Root demo canister that orchestrates the sample canisters in the reference
//! topology.
//!

#![expect(clippy::unused_async)]

use canic::api::auth::AuthApi;

//
// CANIC
//

canic::start_root!();

// Publish root auth material before the first live delegated-auth request path.
async fn canic_setup() {
    let _ = AuthApi::publish_root_auth_material().await;
}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
