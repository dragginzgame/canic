#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::AUTH;

//
// CANIC
//

canic::start!(AUTH);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
