use crate::{
    Error,
    ic::call::Call,
    interface::{self, InterfaceError, ic::IcError, response::Response},
};
use candid::{CandidType, Principal, encode_one};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

///
/// RequestError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum RequestError {
    #[error("invalid response type")]
    InvalidResponseType,
}

///
/// Request
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    CanisterCreate(CanisterCreate),
    CanisterUpgrade(CanisterUpgrade),
}

///
/// CanisterCreate
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct CanisterCreate {
    pub path: String,
    pub extra: Option<Vec<u8>>,
}

///
/// CanisterUpgrade
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct CanisterUpgrade {
    pub pid: Principal,
    pub path: String,
}

///
/// Cycles
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct Cycles {
    pub cycles: u128,
}

///
/// REQUEST
///

// request
pub async fn request(request: Request) -> Result<Response, Error> {
    let root_pid = crate::interface::memory::canister::state::get_root_pid()?;
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

// canister_create
// create a Request and pass it to the request shared endpoint
pub async fn canister_create(path: &str) -> Result<Principal, Error> {
    let req = Request::CanisterCreate(CanisterCreate {
        path: path.to_string(),
        extra: None,
    });

    match request(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_pid) => {
                // success, update child index
                interface::memory::canister::child_index::insert_canister(new_pid, path);

                Ok(new_pid)
            }
            Response::CanisterUpgrade => Err(InterfaceError::RequestError(
                RequestError::InvalidResponseType,
            ))?,
        },
        Err(e) => Err(e),
    }
}

// canister_create_arg
pub async fn canister_create_arg<A>(path: &str, extra: A) -> Result<Principal, Error>
where
    A: CandidType + Send + Sync + Serialize + DeserializeOwned,
{
    let encoded = encode_one(extra)
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    let req = Request::CanisterCreate(CanisterCreate {
        path: path.to_string(),
        extra: Some(encoded),
    });

    match request(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_pid) => {
                // success, update child index
                interface::memory::canister::child_index::insert_canister(new_pid, path);

                Ok(new_pid)
            }
            Response::CanisterUpgrade => Err(InterfaceError::RequestError(
                RequestError::InvalidResponseType,
            ))?,
        },
        Err(e) => Err(e),
    }
}

// canister_upgrade
pub async fn canister_upgrade(pid: Principal, path: &str) -> Result<(), Error> {
    let req = Request::CanisterUpgrade(CanisterUpgrade {
        pid,
        path: path.to_string(),
    });

    let _res = request(req).await?;

    Ok(())
}
