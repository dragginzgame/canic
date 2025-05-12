use crate::{
    Error,
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
        RequestKind::CanisterCreate(kind) => create_canister(&kind.name).await,
        RequestKind::CanisterUpgrade(kind) => upgrade_canister(kind.pid, &kind.name).await,
    }
}

// create_canister
async fn create_canister(path: &str) -> Result<Response, Error> {
    let bytes = crate::interface::state::wasm::get_wasm(path)?;
    let new_canister_id =
        crate::interface::ic::create_canister(path, bytes, vec![], msg_caller()).await?;

    // update subnet index
    //  if !path.is_sharded() {
    //      SUBNET_INDEX.with_borrow_mut(|this| this.set_canister(path, new_canister_id));
    //  }

    Ok(Response::CanisterCreate(new_canister_id))
}

// upgrade_canister
async fn upgrade_canister(pid: Principal, path: &str) -> Result<Response, Error> {
    let bytes = interface::state::wasm::get_wasm(path)?;
    interface::ic::upgrade_canister(pid, bytes).await?;

    Ok(Response::CanisterUpgrade)
}
