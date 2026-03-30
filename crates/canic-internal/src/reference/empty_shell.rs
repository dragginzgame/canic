#![allow(clippy::unused_async)]

///
/// canic_setup
///
/// Shared no-op setup hook for reference canisters that only exercise the base runtime.
pub async fn canic_setup() {}

///
/// canic_install
///
/// Shared no-op install hook for reference canisters that do not need install payload handling.
pub async fn canic_install(_: Option<Vec<u8>>) {}

///
/// canic_upgrade
///
/// Shared no-op upgrade hook for reference canisters that only validate lifecycle wiring.
pub async fn canic_upgrade() {}
