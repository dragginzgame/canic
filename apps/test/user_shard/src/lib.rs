#![expect(clippy::unused_async)]

mod fixture_importer;
mod reinstall_fixture;

mod protocol;

use canic::{Error, prelude::*};
use std::cell::RefCell;

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

/// Prove direct Fleet-admitted ingress for managed child qualification.
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

/// Drive the maintained durable top-up request with an optional lost reply.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::TOPUP_REQUEST, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_topup_request(
    request: protocol::TopupRequest,
) -> Result<([u8; 32], Option<canic::dto::rpc::CyclesResponse>), Error> {
    canic::__internal::core::api::runtime::topup_fixture::request(
        request.cycles,
        request.discard_reply,
    )
    .await
}

/// Prove the parent still rejects a sibling reusing another actor's operation.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::TOPUP_COLLISION, request = protocol::OPERATION_REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_topup_collision(
    request: protocol::TopupCollision,
) -> Result<canic::dto::rpc::CyclesResponse, Error> {
    canic::__internal::core::api::runtime::topup_fixture::collide(
        request.cycles,
        request.operation_id,
    )
    .await
}

/// Put a disposable Shard below demand and reconcile its existing production timer.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::TOPUP_DEMAND, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_topup_demand(retain: u128) -> Result<(), Error> {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    ic_platform::cdk::api::cycles_burn(excess);
    canic::__internal::core::api::runtime::root_funding::RootFundingTimerApi::reconcile()
}

/// Burn disposable fixture cycles before one real parent request fails admission.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::TOPUP_EXHAUSTION, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_topup_exhaustion(
    retain: u128,
) -> Result<([u8; 32], Option<canic::dto::rpc::CyclesResponse>), Error> {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    ic_platform::cdk::api::cycles_burn(excess);
    canic::__internal::core::api::runtime::topup_fixture::request(1_000_000_000_000, false).await
}

/// Deplete a disposable Shard after retaining its terminal diagnostic.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::RECOVERY_BALANCE, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_recovery_balance(retain: u128) -> Result<u128, Error> {
    let excess = ic_platform::cdk::api::canister_cycle_balance().saturating_sub(retain);
    Ok(ic_platform::cdk::api::cycles_burn(excess))
}

canic::finish!();
