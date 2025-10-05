//! Helpers that build requests routed through the root canister.
//!
//! Non-root canisters submit orchestration requests to root using the
//! `canic_response` endpoint. This module owns the request envelope and
//! high-level helpers for creating new canisters, triggering upgrades, or
//! moving cycles between principals.

use crate::{
    Error,
    cdk::call::Call,
    memory::{context::CanisterContext, state::CanisterState, topology::SubnetChildren},
    ops::{
        prelude::*,
        response::{CreateCanisterResponse, CyclesResponse, Response, UpgradeCanisterResponse},
    },
};
use candid::encode_one;
use thiserror::Error as ThisError;

/// Errors produced during request dispatch or response handling.
#[derive(Debug, ThisError)]
pub enum RequestError {
    #[error("this request is not allowed to be called on root")]
    RootNotAllowed,

    #[error("invalid response type")]
    InvalidResponseType,
}

/// Root-directed orchestration commands.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

/// Payload for [`Request::CreateCanister`].
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_type: CanisterType,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

/// Parent-location choices for a new canister.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CreateCanisterParent {
    Root,
    Caller,
    Canister(Principal),
    Directory(CanisterType),
}

/// Payload for [`Request::UpgradeCanister`].
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    pub canister_type: CanisterType,
}

/// Payload for [`Request::Cycles`].
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
}

/// Send a request to the root canister and decode its response.
async fn request(request: Request) -> Result<Response, Error> {
    let root_pid = CanisterContext::try_get_root_pid()?;

    let call_response = Call::unbounded_wait(root_pid, "canic_response")
        .with_arg(&request)
        .await?;

    call_response.candid::<Result<Response, Error>>()?
}

/// Ask root to create and install a canister of the given type.
pub async fn create_canister_request<A>(
    canister_type: &CanisterType,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    let encoded = match extra {
        Some(extra) => Some(encode_one(extra)?),
        None => None,
    };

    // build request
    let q = Request::CreateCanister(CreateCanisterRequest {
        canister_type: canister_type.clone(),
        parent,
        extra_arg: encoded,
    });

    match request(q).await? {
        Response::CreateCanister(res) => Ok(res),
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}

/// Ask root to upgrade a child canister to its latest registered WASM.
pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    // check this is a valid child
    let canister = SubnetChildren::try_find_by_pid(&canister_pid)?;

    // send the request
    let q = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: canister.pid,
        canister_type: canister.ty,
    });

    match request(q).await? {
        Response::UpgradeCanister(res) => Ok(res),
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}

/// Request a cycle transfer from root to the current canister.
pub async fn cycles_request(cycles: u128) -> Result<CyclesResponse, Error> {
    let q = Request::Cycles(CyclesRequest { cycles });

    if CanisterState::is_root() {
        return Err(OpsError::RequestError(RequestError::RootNotAllowed))?;
    }

    match request(q).await? {
        Response::Cycles(res) => Ok(res),
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}
