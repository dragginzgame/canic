#![expect(clippy::unused_async)]

canic::start_root!();

pub async fn canic_setup() {}

pub async fn canic_install() {}

pub async fn canic_upgrade() {}

canic::cdk::export_candid_debug!();
