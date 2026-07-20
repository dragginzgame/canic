#![expect(clippy::unused_async)]

use canic::api::canister::placement::ShardingApi;
use canic::{Error, cdk::types::Principal, prelude::*};
use std::cell::RefCell;

const POOL_NAME: &str = "user_shards";

thread_local! {
    static RECOVERY_GENERATION: RefCell<String> = const { RefCell::new(String::new()) };
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Create one user shard assignment for the provided principal.
#[canic_update(public)]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    canic::access::require_local()?;
    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string()).await
}

/// Dry-run the user-shard placement decision using config-driven policy.
#[canic_query(public)]
async fn plan_create_account(pid: Principal) -> Result<String, Error> {
    canic::access::require_local()?;
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, pid.to_string())?;

    Ok(format!("{plan:?}"))
}

/// Set deterministic fixture state for the disposable backup/restore journey.
#[canic_update(public)]
async fn test_set_recovery_generation(generation: String) -> Result<(), Error> {
    RECOVERY_GENERATION.with_borrow_mut(|current| *current = generation);
    Ok(())
}

/// Return deterministic fixture state for the disposable backup/restore journey.
#[canic_query(public)]
async fn test_recovery_generation() -> Result<String, Error> {
    Ok(RECOVERY_GENERATION.with_borrow(Clone::clone))
}

canic::finish!();
