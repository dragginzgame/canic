//!
//! Minimal demo canister used as the smallest Canic reference baseline.
//! Lives in `fleets` solely as a lightweight shell for audit and test flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![expect(clippy::unused_async)]

use canic::ids::CanisterRole;

const MINIMAL: CanisterRole = CanisterRole::new("minimal");

/// Run no-op setup for the minimal reference shell.
pub async fn canic_setup() {}

/// Accept no install payload for the minimal reference shell.
pub async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the minimal reference shell.
pub async fn canic_upgrade() {}

//
// CANIC
//

canic::start!(MINIMAL);

canic::finish!();
