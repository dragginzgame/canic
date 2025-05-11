use crate::{
    ic::call::{Call, Error as CallError},
    interface::{
        self, InterfaceError,
        ic::IcError,
        response::{Response, ResponseError},
    },
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
    #[error("call error: {0}")]
    CallError(String),

    #[error("invalid response type")]
    InvalidResponseType,

    #[error(transparent)]
    IcError(#[from] IcError),

    #[error(transparent)]
    ResponseError(#[from] ResponseError),

    #[error(transparent)]
    StateError(#[from] StateError),

    #[error(transparent)]
    WasmError(#[from] WasmError),
}

impl From<CallError> for RequestError {
    fn from(error: CallError) -> Self {
        Self::CallError(error.to_string())
    }
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
    pub const fn new_canister_create(ty: &str) -> Self {
        Self {
            kind: RequestKind::CanisterCreate(CanisterCreate { ty }),
        }
    }

    #[must_use]
    pub const fn new_canister_upgrade(pid: Principal, ty: &str) -> Self {
        Self {
            kind: RequestKind::CanisterUpgrade(CanisterUpgrade { pid, ty }),
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
    pub ty: String,
}

///
/// CanisterUpgrade
///

#[derive(CandidType, Clone, Debug, Display, Serialize, Deserialize)]
#[display("pid, ty")]
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
pub async fn request_api(request: Request) -> Result<Response, InterfaceError> {
    let root_pid = interface::state::canister_state::get_root_id_api()?;
    let res = Call::unbounded_wait(root_pid, "response")
        .with_arg(&request)
        .await
        .map_err(IcError::from)?;

    res.candid::<Result<Response, InterfaceError>>()
        .map_err(IcError::from)?
}

// canister_create_api
// create a Request and pass it to the request shared endpoint
#[allow(clippy::unnecessary_map_on_constructor)]
pub async fn canister_create_api(ty: CanisterType) -> Result<Principal, InterfaceError> {
    let req = Request::new_canister_create(ty.clone());

    match request_api(req).await {
        Ok(response) => match response {
            Response::CanisterCreate(new_pid) => {
                // success, update child index
                interface::state::core::child_index::insert_canister(new_pid, ty.clone());

                // cascade subnet_index after each new canister
                if !ty.is_sharded() {
                    crate::interface::cascade::subnet_index_cascade_api().await?;
                }

                Ok(new_pid)
            }
            Response::CanisterUpgrade => {
                Err(RequestError::InvalidResponseType).map_err(InterfaceError::RequestError)?
            }
        },
        Err(e) => Err(e),
    }
}

// canister_upgrade_api
pub async fn canister_upgrade_api(pid: Principal, ty: CanisterType) -> Result<(), InterfaceError> {
    let req = Request::new_canister_upgrade(pid, ty);
    let _res = request_api(req).await?;

    Ok(())
}
