use crate::{
    Error, Log,
    ic::{api::msg_caller, call::Call},
    interface::{
        self, InterfaceError,
        ic::{IcError, create_canister},
        request::Request,
        state::root::canister_registry,
    },
    log,
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
        Request::CanisterCreate(req) => canister_create(&req.path, req.extra).await,
        Request::CanisterUpgrade(req) => canister_upgrade(req.pid, &req.path).await,
    }
}

// canister_create
async fn canister_create(path: &str, extra: Option<Vec<u8>>) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;
    let parent_pid = msg_caller();
    let controllers = vec![root_pid];

    log!(Log::Warn, "canister_create: {path:?} {extra:?}");

    let arg = match extra {
        Some(extra) => encode_args((root_pid, parent_pid, extra)),
        None => encode_args((root_pid, parent_pid)),
    }
    .map_err(IcError::from)
    .map_err(InterfaceError::from)?;

    // create the canister
    let new_canister_id = create_canister(path, canister.wasm, controllers, arg).await?;

    // call init_async with the extra param
    // match extra {
    //     Some(extra) => Call::unbounded_wait(new_canister_id, "init_async").take_raw_args(extra),
    //     None => Call::unbounded_wait(new_canister_id, "init_async"),
    // }
    // .await
    // .map_err(IcError::from)
    // .map_err(InterfaceError::IcError)?;

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
