#![allow(clippy::unused_async)]

use canic::prelude::*;
use canic_internal::canister::APP;

//
// CANIC
//

canic::start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
