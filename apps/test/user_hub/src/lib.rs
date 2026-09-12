#![expect(clippy::unused_async)]

mod fixture_importer;
mod reinstall_fixture;

use candid::Principal;
use canic::api::canister::placement::ShardingApi;
use canic::{Error, prelude::*};
use std::cell::RefCell;

const POOL_NAME: &str = "user_shards";

thread_local! {
    static RECOVERY_GENERATION: RefCell<String> = const { RefCell::new(String::new()) };
}

canic::start!(lifecycle_participant(
    init = fixture_importer::restore,
    post_upgrade = fixture_importer::restore
),);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {
    reinstall_fixture::seed();
}
async fn canic_upgrade() {}

/// Create one user shard assignment for the provided principal.
#[canic_update(requires(env::build_local_only()))]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string()).await
}

/// Exercise a distinct, caller-retained child operation after a fixture Shard is recycled.
#[canic_update(requires(caller::is_controller()))]
async fn test_create_fixture_child(operation_id: [u8; 32]) -> Result<Principal, Error> {
    canic::api::rpc::RpcApi::create_canister_request::<Vec<u8>>(
        operation_id,
        &"user_shard".into(),
        canic::dto::rpc::CreateCanisterParent::ThisCanister,
        None,
    )
    .await
    .map(|response| response.new_canister_pid)
}

/// Dry-run the user-shard placement decision using config-driven policy.
#[canic_query(requires(env::build_local_only()))]
async fn plan_create_account(pid: Principal) -> Result<String, Error> {
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, pid.to_string())?;

    Ok(format!("{plan:?}"))
}

/// Prove direct Fleet-admitted ingress for managed Component Group qualification.
#[canic_query(requires(caller::is_fleet_admitted()))]
async fn test_fleet_admission_probe() -> Result<Principal, Error> {
    Ok(ic_cdk::api::msg_caller())
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

/// Write a user-owned stable row in the disposable reinstall fixture.
#[canic_update(public)]
async fn test_set_user_row(id: u64, value: u64) -> Result<(), Error> {
    reinstall_fixture::insert(id, value);
    Ok(())
}

/// Inspect application rows, including the authored system fixture at key zero.
#[canic_query(public)]
async fn test_user_rows() -> Result<Vec<reinstall_fixture::UserRow>, Error> {
    Ok(reinstall_fixture::rows())
}

canic::finish!();
