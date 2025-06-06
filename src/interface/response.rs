use crate::{
    Error,
    ic::api::msg_caller,
    interface::{
        self,
        request::{Request, RequestKind},
        state::root::canister_registry,
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
        RequestKind::CanisterCreate(kind) => create_canister(&kind.path).await,
        RequestKind::CanisterUpgrade(kind) => upgrade_canister(kind.pid, &kind.path).await,
    }
}

// create_canister
async fn create_canister(path: &str) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;

    let new_canister_id =
        crate::interface::ic::create_canister(path, canister.wasm, vec![root_pid], msg_caller())
            .await?;

    // update subnet index
    if !canister.def.is_sharded {
        interface::memory::subnet::index::set_canister(path, new_canister_id);
        interface::cascade::subnet_index_cascade().await?;
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// upgrade_canister
async fn upgrade_canister(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    interface::ic::upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::CanisterUpgrade)
}
