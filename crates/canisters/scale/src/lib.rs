//!
//! Scaling worker demo canister used to exercise the ops scaling helpers.
//! Part of the `crates/canisters` showcase suite.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::SCALE;

//
// CANIC
//

canic::start!(SCALE);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
