//!
//! Shard worker demo canister used when exercising sharding ops flows.
//! Included in `canisters` as sample-only code.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target.
//!

#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::{
    canister::SHARD,
    reference::empty_shell::{canic_install, canic_setup, canic_upgrade},
};

//
// CANIC
//

canic::start!(SHARD);

canic::cdk::export_candid_debug!();
