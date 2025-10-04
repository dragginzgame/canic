#![allow(clippy::unused_async)]

use canic::{canister::SCALE, prelude::*};

//
// ICU
//

canic_start!(SCALE);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

export_candid!();
