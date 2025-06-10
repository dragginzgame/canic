use crate::{
    Error,
    ic::call::Call,
    interface::{self, InterfaceError, ic::IcError, response::Response},
};
use candid::{CandidType, Encode, Principal};
use serde::{Deserialize, Serialize};
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

impl Request {
    pub fn new_canister_create<A>(path: &str, args: Option<A>) -> Result<Self, Error>
    where
        A: CandidType + Send + Sync,
    {
        let encoded_args = Encode!(&args)
            .map_err(IcError::from)
            .map_err(InterfaceError::from)?;

        Ok(Self::CanisterCreate(CanisterCreate {
            path: path.to_string(),
            args: encoded_args,
        }))
    }

    #[must_use]
    pub fn new_canister_upgrade(pid: Principal, path: &str) -> Self {
        Self::CanisterUpgrade(CanisterUpgrade {
            pid,
            path: path.to_string(),
        })
    }
}

///
/// CanisterCreate
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct CanisterCreate {
    pub path: String,
    pub args: Option<Vec<u8>>,
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
pub async fn canister_create<A>(path: &str, args: A) -> Result<Principal, Error>
where
    A: CandidType + Sync + Send,
{
    let req = Request::new_canister_create(path, args)?;

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
    let req = Request::new_canister_upgrade(pid, path);
    let _res = request(req).await?;

    Ok(())
}
