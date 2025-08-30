use crate::{
    Error,
    cdk::call::Call,
    memory::{CanisterChildren, CanisterState, canister::CanisterEntry},
    ops::{
        prelude::*,
        response::{CreateCanisterResponse, CyclesResponse, Response, UpgradeCanisterResponse},
    },
};
use candid::encode_one;
use thiserror::Error as ThisError;

///
/// RequestError
///

#[derive(Debug, ThisError)]
pub enum RequestError {
    #[error("this request is not allowed to be called on root")]
    RootNotAllowed,

    #[error("invalid response type")]
    InvalidResponseType,
}

///
/// Request
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

///
/// CreateCanisterRequest
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_type: CanisterType,
    pub parents: Vec<CanisterEntry>,
    pub extra_arg: Option<Vec<u8>>,
}

///
/// UpgradeCanisterRequest
/// upgrades canister_pid with the canister's wasm
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    pub canister_type: CanisterType,
}

///
/// CyclesRequest
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: Cycles,
}

///
/// REQUEST
///

// request
// sends the request to root::icu_response
async fn request(request: Request) -> Result<Response, Error> {
    let root_pid = CanisterState::get_root_pid();

    let call_response = Call::unbounded_wait(root_pid, "icu_response")
        .with_arg(&request)
        .await?;

    call_response.candid::<Result<Response, Error>>()?
}

// create_canister_request
pub async fn create_canister_request<A>(
    canister_type: &CanisterType,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    let encoded = match extra {
        Some(extra) => Some(encode_one(extra)?),
        None => None,
    };

    // build parents
    let mut parents = CanisterState::get_parents();
    let this = CanisterEntry::this()?;
    parents.push(this);

    // build request
    let q = Request::CreateCanister(CreateCanisterRequest {
        canister_type: canister_type.clone(),
        parents,
        extra_arg: encoded,
    });

    match request(q).await? {
        Response::CreateCanister(res) => {
            // update the local child index
            CanisterChildren::insert(res.new_canister_pid, canister_type.clone());

            Ok(res)
        }
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}

// upgrade_canister_request
pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    // check this is a valid child
    let canister_type = CanisterChildren::try_get(&canister_pid)?;

    // send the request
    let q = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid,
        canister_type,
    });

    match request(q).await? {
        Response::UpgradeCanister(res) => Ok(res),
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}

// cycles_request
pub async fn cycles_request(cycles: Cycles) -> Result<CyclesResponse, Error> {
    let q = Request::Cycles(CyclesRequest { cycles });

    if CanisterState::is_root() {
        return Err(OpsError::RequestError(RequestError::RootNotAllowed))?;
    }

    match request(q).await? {
        Response::Cycles(res) => Ok(res),
        _ => Err(OpsError::RequestError(RequestError::InvalidResponseType))?,
    }
}
