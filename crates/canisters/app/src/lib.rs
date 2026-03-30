//!
//! App demo canister used for local/dev Canic testing.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::{
    canister::APP,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(APP);

canic::cdk::export_candid_debug!();
