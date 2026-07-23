#![expect(clippy::unused_async)]

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::start!();

canic::finish!();
