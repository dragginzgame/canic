//!
//! Scaling worker demo canister used to exercise the ops scaling helpers.
//! Part of the `canisters` showcase suite.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic_internal::{
    canister::SCALE,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(SCALE);

canic::cdk::export_candid_debug!();
