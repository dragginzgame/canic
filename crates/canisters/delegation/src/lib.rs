//!
//! Delegation demo canister exercising session-related flows for testing.
//! Part of the showcase canisters that live under `crates/canisters`.
//!

#![allow(clippy::unused_async)]

use canic::{canister::DELEGATION, prelude::*};

//
// ICU
//

canic_start!(DELEGATION);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
