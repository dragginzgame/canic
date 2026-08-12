//! Minimal database C Canister for Toko-shaped topology qualification.

#![expect(clippy::unused_async)]

canic::start!();

async fn canic_setup() {}

async fn canic_install(_args: Option<Vec<u8>>) {}

async fn canic_upgrade() {}

canic::finish!();
