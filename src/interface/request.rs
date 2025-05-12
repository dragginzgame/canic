use crate::{
    Error,
    ic::call::Call,
    interface::{self, InterfaceError, ic::IcError, response::Response},
};
use candid::{CandidType, Principal};
use derive_more::Display;
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
pub struct Request {
    pub kind: RequestKind,
}

impl Request {
    #[must_use]
    pub fn new_canister_create(name: String) -> Self {
        Self {
            kind: RequestKind::CanisterCreate(CanisterCreate { name }),
        }
    }

    #[must_use]
    pub fn new_canister_upgrade(pid: Principal, name: String) -> Self {
        Self {
            kind: RequestKind::CanisterUpgrade(CanisterUpgrade { pid, name }),
        }
    }
}

///
/// RequestKind
///

#[derive(CandidType, Clone, Debug, Display, Serialize, Deserialize)]
pub enum RequestKind {
    CanisterCreate(CanisterCreate),
    CanisterUpgrade(CanisterUpgrade),
}

///
/// CanisterCreate
///

#[derive(CandidType, Clone, Debug, Display, Serialize, Deserialize)]
pub struct CanisterCreate {
    pub name: String,
}

///
/// CanisterUpgrade
///

#[derive(CandidType, Clone, Debug, Display, Serialize, Deserialize)]
#[display("pid, name")]
pub struct CanisterUpgrade {
    pub pid: Principal,
    pub name: String,
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
/// all types of canister, but root just passes it to response
///

// request_api
pub async fn request_api(request: Request) -> Result<Response, Error> {
    let root_pid = crate::interface::state::core::canister_state::get_root_id()?;
    let res = Call::unbounded_wait(root_pid, "response")
        .with_arg(&request)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    let a = res
        .candid()
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(a)
}

// canister_create_api
// create a Request and pass it to the request shared endpoint
pub async fn canister_create_api(path: &str) -> Result<Principal, Error> {
    let req = Request::new_canister_create(path.to_string());

    match request_api(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_pid) => {
                // success, update child index
                interface::state::core::child_index::insert_canister(new_pid, path.to_string());

                // cascade subnet_index after each new canister
                //        if !path.is_sharded() {
                //            crate::interface::cascade::subnet_index_cascade_api().await?;
                //        }

                Ok(new_pid)
            }
            Response::CanisterUpgrade => Err(InterfaceError::RequestError(
                RequestError::InvalidResponseType,
            ))?,
        },
        Err(e) => Err(e),
    }
}

// canister_upgrade_api
pub async fn canister_upgrade_api(pid: Principal, name: String) -> Result<(), Error> {
    let req = Request::new_canister_upgrade(pid, name);
    let _res = request_api(req).await?;

    Ok(())
}
