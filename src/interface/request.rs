use crate::{
    Error,
    ic::call::Call,
    interface::{
        self, InterfaceError, ic::IcError, response::Response, state::root::canister_registry,
    },
};
use candid::{CandidType, Principal};
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
    pub fn new_canister_create(path: &str) -> Self {
        Self {
            kind: RequestKind::CanisterCreate(CanisterCreate {
                path: path.to_string(),
            }),
        }
    }

    #[must_use]
    pub fn new_canister_upgrade(pid: Principal, path: &str) -> Self {
        Self {
            kind: RequestKind::CanisterUpgrade(CanisterUpgrade {
                pid,
                path: path.to_string(),
            }),
        }
    }
}

///
/// RequestKind
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub enum RequestKind {
    CanisterCreate(CanisterCreate),
    CanisterUpgrade(CanisterUpgrade),
}

///
/// CanisterCreate
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct CanisterCreate {
    pub path: String,
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
    let root_pid = crate::interface::state::core::canister_state::get_root_pid()?;
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
    let req = Request::new_canister_create(path);
    let canister = canister_registry::get_canister(path)?;

    match request(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_pid) => {
                // success, update child index
                interface::state::core::child_index::insert_canister(new_pid, path);

                // cascade subnet_index after each new canister
                if !canister.def.is_sharded {
                    crate::interface::cascade::subnet_index_cascade().await?;
                }

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
