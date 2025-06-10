use crate::{
    Error,
    ic::api::msg_caller,
    interface::{self, request::Request, state::root::canister_registry},
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
    match req {
        Request::CanisterCreate(cc) => canister_create(&cc.path, cc.extra).await,
        Request::CanisterUpgrade(cu) => canister_upgrade(cu.pid, &cu.path).await,
    }
}

// canister_create
async fn canister_create<A>(path: &str, extra: A) -> Result<Response, Error>
where
    A: CandidType + Send + Sync,
{
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;
    let parent_pid = msg_caller();
    let args = (root_pid, parent_pid, extra);

    let new_canister_id =
        crate::interface::ic::create_canister(path, canister.wasm, vec![root_pid], args).await?;

    // update subnet index
    if !canister.def.is_sharded {
        interface::memory::subnet::index::set_canister(path, new_canister_id);
        interface::cascade::subnet_index_cascade().await?;
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// canister_upgrade
async fn canister_upgrade(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    interface::ic::upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::CanisterUpgrade)
}
