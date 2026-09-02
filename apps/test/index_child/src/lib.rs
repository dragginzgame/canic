#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, prelude::*};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Prove direct Fleet-admitted ingress for an index-created managed child.
#[canic_query(requires(caller::is_fleet_admitted()))]
async fn test_fleet_admission_probe() -> Result<Principal, Error> {
    Ok(ic_cdk::api::msg_caller())
}

canic::finish!();
