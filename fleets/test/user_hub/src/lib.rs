#![expect(clippy::unused_async)]

use canic::api::canister::placement::ShardingApi;
use canic::{Error, cdk::types::Principal, prelude::*};

const POOL_NAME: &str = "user_shards";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Create one user shard assignment for the provided principal.
#[canic_update]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    canic::access::require_local()?;
    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string()).await
}

/// Dry-run the user-shard placement decision using config-driven policy.
#[canic_query]
async fn plan_create_account(pid: Principal) -> Result<String, Error> {
    canic::access::require_local()?;
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, pid.to_string())?;

    Ok(format!("{plan:?}"))
}

canic::finish!();
