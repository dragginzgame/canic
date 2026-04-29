//!
//! App demo canister in the reference topology.
//!

#![allow(clippy::unused_async)]

use canic_reference_support::{
    canister::APP,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(APP);

canic::cdk::export_candid_debug!();
