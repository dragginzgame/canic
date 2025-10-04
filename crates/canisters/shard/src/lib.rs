#![allow(clippy::unused_async)]

use canic::{canister::SHARD, prelude::*};

//
// ICU
//

canic_start!(SHARD);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
