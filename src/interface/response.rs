use crate::{
    interface::{
        self, InterfaceError,
        ic::IcError,
        request::{Request, RequestKind},
    },
    state::StateError,
};
use candid::{CandidType, Principal};
use derive_more::Display;
use icu::{state::SUBNET_INDEX, wasm::WasmManager};
use mimic::ic::api::msg_caller;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// ResponseError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum ResponseError {
    #[error(transparent)]
    IcError(#[from] IcError),

    #[error(transparent)]
    StateError(#[from] StateError),
}

///
/// Response
/// the root canister currently is the only one with the response() endpoint
///

#[derive(CandidType, Clone, Debug, Display, Serialize, Deserialize)]
pub enum Response {
    CanisterCreate(Principal),
    CanisterUpgrade,
}

// response
pub async fn response(req: Request) -> Result<Response, InterfaceError> {
    match req.kind {
        RequestKind::CanisterCreate(kind) => create_canister(kind.ty).await,
        RequestKind::CanisterUpgrade(kind) => upgrade_canister(kind.pid, kind.ty).await,
    }
}

// create_canister
async fn create_canister(ty: &str) -> Result<Response, InterfaceError> {
    let bytes = WasmManager::get_wasm(&ty).map_err(ResponseError::IcuError)?;
    let new_canister_id = interface::ic::create_canister(ty.clone(), bytes, msg_caller()).await?;

    // update subnet index
    if !ty.is_sharded() {
        SUBNET_INDEX.with_borrow_mut(|this| this.set_canister(ty, new_canister_id));
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// upgrade_canister
async fn upgrade_canister(pid: Principal, ty: &str) -> Result<Response, InterfaceError> {
    let bytes = WasmManager::get_wasm(&ty).map_err(ResponseError::IcuError)?;
    interface::ic::upgrade_canister(pid, bytes).await?;

    Ok(Response::CanisterUpgrade)
}
