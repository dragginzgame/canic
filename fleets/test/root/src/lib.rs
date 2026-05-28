#![expect(clippy::unused_async)]

use canic::api::auth::AuthApi;

canic::start!();

// Publish root auth material before the first live delegated-auth request path.
async fn canic_setup() {
    let _ = AuthApi::publish_root_auth_material().await;
}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
