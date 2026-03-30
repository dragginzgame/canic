//!
//! Minimal demo canister used as a minimal Canic baseline.
//! Lives in `crates/canisters` solely as a minimal shell for audit and test flows.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::{
    canister::MINIMAL,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(MINIMAL);

canic::cdk::export_candid_debug!();
