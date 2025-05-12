use crate::{
    CanisterDyn, Error,
    ic::api::msg_caller,
    interface::{
        self,
        request::{Request, RequestKind},
    },
};
use candid::{CandidType, Principal};
use derive_more::Display;
use serde::{Deserialize, Serialize};

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
pub async fn response(req: Request) -> Result<Response, Error> {
    match req.kind {
        RequestKind::CanisterCreate(kind) => create_canister(&kind.canister).await,
        RequestKind::CanisterUpgrade(kind) => upgrade_canister(kind.pid, &kind.canister).await,
    }
}

// create_canister
async fn create_canister(canister: &CanisterDyn) -> Result<Response, Error> {
    let path = &canister.path;

    let bytes = interface::state::wasm::get_wasm(path)?;
    let root_pid = interface::state::core::canister_state::get_root_pid()?;

    let new_canister_id =
        crate::interface::ic::create_canister(path, bytes, vec![root_pid], msg_caller()).await?;

    // update subnet index
    if !canister.is_sharded {
        interface::state::core::subnet_index::set_canister(path, new_canister_id);
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// upgrade_canister
async fn upgrade_canister(pid: Principal, canister: &CanisterDyn) -> Result<Response, Error> {
    let bytes = interface::state::wasm::get_wasm(&canister.path)?;
    interface::ic::upgrade_canister(pid, bytes).await?;

    Ok(Response::CanisterUpgrade)
}
