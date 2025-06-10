use crate::{
    Error,
    ic::{api::msg_caller, call::Call},
    interface::{
        self, InterfaceError,
        ic::{IcError, create_canister},
        request::Request,
        state::root::canister_registry,
    },
};
use candid::{CandidType, Principal, encode_args};
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
async fn canister_create(path: &str, extra: Option<Vec<u8>>) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;
    let parent_pid = msg_caller();
    let controllers = vec![root_pid];

    // format args
    let arg = encode_args((root_pid, parent_pid))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // create the canister
    let new_canister_id = create_canister(path, canister.wasm, controllers, arg).await?;

    // call _init with the extra param
    match extra {
        Some(arg) => Call::unbounded_wait(new_canister_id, "_init").with_arg(&arg),
        None => Call::unbounded_wait(new_canister_id, "_init"),
    }
    .await
    .map_err(IcError::from)
    .map_err(InterfaceError::IcError)?;

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
