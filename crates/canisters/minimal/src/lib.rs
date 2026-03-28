//!
//! Minimal demo canister used as a minimal Canic baseline.
//! Lives in `crates/canisters` solely as a minimal shell for audit and test flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::MINIMAL;

//
// CANIC
//

canic::start!(MINIMAL);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[cfg(debug_assertions)]
export_candid!();
