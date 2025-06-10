use crate::{
    Error, Log,
    ic::api::msg_caller,
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

macro_rules! encode_args_tuple {
    ($first:expr, $second:expr) => {
        encode_args(($first, $second))
    };
    ($first:expr, $second:expr, $third:expr) => {
        encode_args(($first, $second, $third))
    };
    ($first:expr, $second:expr, $third:expr, $fourth:expr) => {
        encode_args(($first, $second, $third, $fourth))
    };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenericValue {
    Principal(Principal),
    U64(u64),
    // TODO: add more types
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
pub async fn response(req: Request) -> Result<Response, Error> {
    match req {
        Request::CanisterCreate(cc) => canister_create(&cc.path, cc.extra).await,
        Request::CanisterCreateWithArg(cc) => canister_create_with_arg(&cc.path, cc.arg).await,
        Request::CanisterUpgrade(cu) => canister_upgrade(cu.pid, &cu.path).await,
    }
}

// canister_create
async fn canister_create<A>(path: &str, extra: Option<A>) -> Result<Response, Error>
where
    A: CandidType + Send + Sync,
{
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;
    let parent_pid = msg_caller();
    let controllers = vec![root_pid];

    // format args
    let arg = match extra {
        Some(extra_arg) => encode_args((root_pid, parent_pid, extra_arg)),
        None => encode_args((root_pid, parent_pid)),
    }
    .map_err(IcError::from)
    .map_err(InterfaceError::from)?;

    log!(Log::Warn, "arg is {arg:?}");

    // create the canister
    let new_canister_id = create_canister(path, canister.wasm, controllers, arg).await?;

    // update subnet index
    if !canister.def.is_sharded {
        interface::memory::subnet::index::set_canister(path, new_canister_id);
        interface::cascade::subnet_index_cascade().await?;
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// canister_create_with_arg
async fn canister_create_with_arg(path: &str, arg: GenericValue) -> Result<Response, Error> {
    let canister = canister_registry::get_canister(path)?;
    let root_pid = interface::memory::canister::state::get_root_pid()?;
    let parent_pid = msg_caller();
    let controllers = vec![root_pid];

    // Keep possibility to be flexible with argument types
    // could potentially work with a Vec<GenericValue> + macro
    let args = match arg {
        GenericValue::Principal(p) => encode_args_tuple!(root_pid, parent_pid, p),
        GenericValue::U64(n) => encode_args_tuple!(root_pid, parent_pid, n),
    }
    .map_err(IcError::from)
    .map_err(InterfaceError::from)?;

    log!(Log::Warn, "arg is {arg:?}");

    // create the canister
    let new_canister_id = create_canister(path, canister.wasm, controllers, args).await?;

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
