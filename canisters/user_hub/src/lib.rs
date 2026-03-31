//!
//! User hub canister that initiates user shard provisioning for placement only.
//!
//! Test-only helper: this canister is intended for local/dev flows and is not
//! a public-facing deployment target. Its endpoints may intentionally omit
//! production-grade auth because they are exercised only in controlled tests.
//!

#![allow(clippy::unused_async)]

use canic::api::canister::placement::ShardingApi;
use canic::{Error, cdk::types::Principal, prelude::*};
use canic_internal::canister::USER_HUB;

const POOL_NAME: &str = "user_shards";

//
// CANIC
//

canic::start!(USER_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// create_account
/// Test-only: no public auth guarantees; intended for local/dev Canic tests.
#[canic_update]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    // Test-only guard: keep this endpoint out of non-local flows.
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
    }

    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string()).await
}

canic::cdk::export_candid_debug!();
