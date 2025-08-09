use crate::{
    Error,
    ic::call::Call,
    interface::{InterfaceError, ic::IcError, root::response::Response},
    memory::{CanisterState, ChildIndex, canister::CanisterParent},
};
use candid::{CandidType, Principal, encode_one};
use serde::Deserialize;
use thiserror::Error as ThisError;

///
/// RequestError
///

#[derive(CandidType, Debug, Deserialize, ThisError)]
pub enum RequestError {
    #[error("invalid response type")]
    InvalidResponseType,
}

///
/// Request
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterArgs),
    UpgradeCanister(UpgradeCanisterArgs),
}

///
/// CreateCanisterArgs
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterArgs {
    pub kind: String,
    pub parents: Vec<CanisterParent>,
    pub extra: Option<Vec<u8>>,
}

///
/// UpgradeCanisterArgs
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterArgs {
    pub pid: Principal,
    pub kind: String,
}

///
/// REQUEST
///

// request
// sends the request to root::icu_response
pub async fn request(request: Request) -> Result<Response, Error> {
    let root_pid = CanisterState::get_root_pid();

    let call_response = Call::unbounded_wait(root_pid, "icu_response")
        .with_arg(&request)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    call_response
        .candid::<Result<Response, Error>>()
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?
}

// create_canister_request
pub async fn create_canister_request<A>(kind: &str, extra: Option<A>) -> Result<Principal, Error>
where
    A: CandidType + Send + Sync,
{
    let encoded = match extra {
        Some(extra) => Some(
            encode_one(extra)
                .map_err(IcError::from)
                .map_err(InterfaceError::from)?,
        ),
        None => None,
    };

    crate::log!(crate::Log::Info, "create_canister_request: {kind}");

    // build parents
    let mut parents = CanisterState::get_parents();
    let this = CanisterParent::this()?;
    parents.push(this);

    // build request
    let req = Request::CreateCanister(CreateCanisterArgs {
        kind: kind.to_string(),
        parents,
        extra: encoded,
    });

    match request(req).await {
        Ok(response) => match response {
            Response::CreateCanister(new_canister_pid) => {
                // update the local child index
                ChildIndex::insert(new_canister_pid, kind);

                Ok(new_canister_pid)
            }
            Response::UpgradeCanister => Err(InterfaceError::RequestError(
                RequestError::InvalidResponseType,
            ))?,
        },
        Err(e) => Err(e),
    }
}

// upgrade_canister_request
pub async fn upgrade_canister_request(pid: Principal) -> Result<(), Error> {
    // check this is a valid child
    let kind = ChildIndex::try_get(&pid)?;

    // send the request
    let req = Request::UpgradeCanister(UpgradeCanisterArgs { pid, kind });
    let _res = request(req).await?;

    Ok(())
}
