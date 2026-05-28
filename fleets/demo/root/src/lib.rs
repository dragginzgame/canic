#![expect(clippy::unused_async)]

canic::start_root!();

async fn canic_setup() {}

async fn canic_install() {}

async fn canic_upgrade() {}

canic::finish!();
