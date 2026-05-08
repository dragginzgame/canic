//!
//! App demo canister in the reference topology.
//!

#![allow(clippy::unused_async)]

use canic::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

/// Run no-op setup for the reference app shell.
pub async fn canic_setup() {}

/// Accept no install payload for the reference app shell.
pub async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the reference app shell.
pub async fn canic_upgrade() {}

//
// CANIC
//

canic::start!(APP);

canic::cdk::export_candid_debug!();
