//! Root-side handlers that fulfil orchestration requests.
//!
//! The root canister exposes `canic_response`, which accepts a
//! [`Request`](crate::ops::request::Request) and returns a [`Response`]. This
//! module contains the implementations for create/upgrade/cycle flows plus the
//! corresponding response payloads.

use crate::{
    Error,
    interface::ic::{deposit_cycles, upgrade_canister},
    memory::topology::SubnetCanisterRegistry,
    ops::{
        canister::create_and_install_canister,
        prelude::*,
        request::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, Request,
            UpgradeCanisterRequest,
        },
    },
    state::wasm::WasmRegistry,
};

///
/// Response
/// Response payloads produced by root for orchestration requests.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}

/// Handle a root-bound orchestration request and produce a [`Response`].
pub async fn response(req: Request) -> Result<Response, Error> {
    OpsError::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}

// create_canister_response
async fn create_canister_response(req: &CreateCanisterRequest) -> Result<Response, Error> {
    // Look up parent
    let parent_pid = match &req.parent {
        CreateCanisterParent::Root => canister_self(),
        CreateCanisterParent::Caller => msg_caller(),
        CreateCanisterParent::Directory(ty) => SubnetCanisterRegistry::try_get_type(ty)?.pid,
        CreateCanisterParent::Canister(pid) => *pid,
    };

    let new_canister_pid =
        create_and_install_canister(&req.canister_type, parent_pid, req.extra_arg.clone()).await?;

    Ok(Response::CreateCanister(CreateCanisterResponse {
        new_canister_pid,
    }))
}

// upgrade_canister_response
async fn upgrade_canister_response(req: &UpgradeCanisterRequest) -> Result<Response, Error> {
    let wasm = WasmRegistry::try_get(&req.canister_type)?;
    upgrade_canister(req.canister_pid, wasm.bytes()).await?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;

    Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
}
