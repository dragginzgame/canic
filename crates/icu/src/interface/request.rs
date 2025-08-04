use crate::{
    Error,
    ic::call::Call,
    interface::{InterfaceError, ic::IcError, response::Response},
    memory::{CanisterState, ChildIndex, canister::CanisterParent},
};
use candid::{CandidType, Principal, encode_one};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// RequestError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum RequestError {
    #[error("invalid response type")]
    InvalidResponseType,
}

///
/// Request
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Request {
    CanisterCreate(CanisterCreate),
    CanisterUpgrade(CanisterUpgrade),
}

///
/// CanisterCreate
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterCreate {
    pub kind: String,
    pub parents: Vec<CanisterParent>,
    pub controllers: Vec<Principal>,
    pub extra: Option<Vec<u8>>,
}

///
/// CanisterUpgrade
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterUpgrade {
    pub pid: Principal,
    pub kind: String,
}

///
/// Cycles
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct Cycles {
    pub cycles: u128,
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

// canister_create_request
pub async fn canister_create_request<A>(
    kind: &str,
    controllers: &[Principal],
    extra: Option<A>,
) -> Result<Principal, Error>
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

    // build parents
    let mut parents = CanisterState::get_parents();
    let this = CanisterParent::this()?;
    parents.push(this);

    // build request
    let req = Request::CanisterCreate(CanisterCreate {
        kind: kind.to_string(),
        parents,
        controllers: controllers.to_vec(),
        extra: encoded,
    });

    match request(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_canister_pid) => {
                // update child index
                ChildIndex::insert(new_canister_pid, kind);

                Ok(new_canister_pid)
            }
            Response::CanisterUpgrade => Err(InterfaceError::RequestError(
                RequestError::InvalidResponseType,
            ))?,
        },
        Err(e) => Err(e),
    }
}

// canister_upgrade_request
pub async fn canister_upgrade_request(pid: Principal) -> Result<(), Error> {
    // check this is a valid child
    let kind = ChildIndex::try_get(&pid)?;

    // send the request
    let req = Request::CanisterUpgrade(CanisterUpgrade { pid, kind });
    let _res = request(req).await?;

    Ok(())
}
