//!
//! Minimal metrics demo canister used to compare the metrics-enabled Canic
//! reference baseline against `canister_minimal`.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![expect(clippy::unused_async)]

/// Run no-op setup for the minimal metrics reference shell.
async fn canic_setup() {}

/// Accept no install payload for the minimal metrics reference shell.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the minimal metrics reference shell.
async fn canic_upgrade() {}

//
// CANIC
//

canic::start!();

canic::finish!();
