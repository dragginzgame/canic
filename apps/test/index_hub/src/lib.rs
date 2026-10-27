#![expect(clippy::unused_async)]

mod ops;
mod protocol;

use canic::{Error, api::canister::placement::PlacementIndexApi, prelude::*};

const POOL_NAME: &str = "items";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Resolve one indexed key, creating its exact managed child when absent.
#[canic_update(requires(env::build_local_only()), wire(id = protocol::RESOLVE_ITEM, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn resolve_item(key: String) -> Result<protocol::IndexStatus, Error> {
    PlacementIndexApi::resolve_or_create(POOL_NAME, key)
        .await
        .map(Into::into)
}

/// Retain one child operation while its bounded application hook remains pending.
#[canic_update(requires(env::build_local_only(), caller::is_controller()), wire(id = protocol::CREATE_STARTUP_CHILD, request = protocol::OPERATION_REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn test_create_startup_child(
    request: protocol::StartupRequest,
) -> Result<ic_platform::Principal, Error> {
    canic::api::rpc::RpcApi::create_canister_request(
        request.operation_id,
        &"index_child".into(),
        canic::dto::rpc::CreateCanisterParent::ThisCanister,
        Some(request.rounds),
    )
    .await
    .map(|response| response.new_canister_pid.into())
}

canic::finish!();
