#![expect(clippy::unused_async)]

use canic::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

pub async fn canic_setup() {}

pub async fn canic_install(_: Option<Vec<u8>>) {}

pub async fn canic_upgrade() {}

canic::start!(APP);

canic::cdk::export_candid_debug!();
