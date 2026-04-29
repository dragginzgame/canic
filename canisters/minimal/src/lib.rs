//!
//! Minimal demo canister used as the smallest Canic reference baseline.
//! Lives in `canisters` solely as a lightweight shell for audit and test flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic_reference_support::{
    canister::MINIMAL,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(MINIMAL);

canic::cdk::export_candid_debug!();
