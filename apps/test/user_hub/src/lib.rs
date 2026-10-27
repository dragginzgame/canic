#![expect(clippy::unused_async)]

mod caller_authority;
mod fixture_importer;
mod reinstall_fixture;

mod protocol;

use canic::api::canister::placement::ShardingApi;
use canic::{Error, prelude::*};
use ic_platform::Principal;
use std::cell::RefCell;

const POOL_NAME: &str = "user_shards";

thread_local! {
    static RECOVERY_GENERATION: RefCell<String> = const { RefCell::new(String::new()) };
}

canic::start!(
    memory = reinstall_fixture::memory(),
    lifecycle_participant(
        init = fixture_importer::restore,
        post_upgrade = fixture_importer::restore
    ),
);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {
    reinstall_fixture::seed();
}
async fn canic_upgrade() {}

/// Create one user shard assignment for the provided principal.
#[canic_update(requires(env::build_local_only()), wire(id = protocol::CREATE_ACCOUNT, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn create_account(pid: Principal) -> Result<Principal, Error> {
    ShardingApi::assign_to_pool(POOL_NAME, pid.to_string())
        .await
        .map(Into::into)
}

/// Exercise a distinct, caller-retained child operation after a fixture Shard is recycled.
#[canic_update(requires(caller::is_controller()), wire(id = protocol::CREATE_FIXTURE_CHILD, request = protocol::OPERATION_REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_create_fixture_child(operation_id: [u8; 32]) -> Result<Principal, Error> {
    canic::api::rpc::RpcApi::create_canister_request::<Vec<u8>>(
        operation_id,
        &"user_shard".into(),
        canic::dto::rpc::CreateCanisterParent::ThisCanister,
        None,
    )
    .await
    .map(|response| response.new_canister_pid.into())
}

/// Dry-run the user-shard placement decision using config-driven policy.
#[canic_query(requires(env::build_local_only()), wire(id = protocol::PLAN_ACCOUNT, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn plan_create_account(pid: Principal) -> Result<String, Error> {
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, pid.to_string())?;

    Ok(format!("{plan:?}"))
}

/// Prove direct Fleet-admitted ingress for managed Component Group qualification.
#[canic_query(requires(caller::is_fleet_admitted()), wire(id = protocol::ADMISSION, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_fleet_admission_probe() -> Result<ic_platform::Principal, Error> {
    Ok(ic_platform::cdk::api::msg_caller())
}

/// Return deterministic fixture state for the disposable backup/restore journey.
#[canic_query(public, wire(id = protocol::GENERATION, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_recovery_generation() -> Result<String, Error> {
    Ok(RECOVERY_GENERATION.with_borrow(Clone::clone))
}

/// Write a user-owned stable row in the disposable reinstall fixture.
#[canic_update(public, wire(id = protocol::SET_ROW, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_set_user_row(row: reinstall_fixture::UserRow) -> Result<(), Error> {
    reinstall_fixture::insert(row.id, row.value);
    Ok(())
}

/// Inspect application rows, including the authored system fixture at key zero.
#[canic_query(public, wire(id = protocol::ROWS, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_user_rows() -> Result<Vec<reinstall_fixture::UserRow>, Error> {
    Ok(reinstall_fixture::rows())
}

/// Spend real fixture cycles before reviewing an underfunded reinstall.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::RECOVERY_BALANCE, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_recovery_balance(retain: u128) -> Result<u128, Error> {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    Ok(ic_platform::cdk::api::cycles_burn(excess))
}

canic::finish!();
