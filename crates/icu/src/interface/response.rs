use crate::{
    Error,
    config::Config,
    interface::{
        InterfaceError,
        ic::{IcError, ic_create_canister, ic_upgrade_canister},
        request::Request,
    },
    memory::{CanisterState, canister::CanisterParent},
    state::CanisterRegistry,
};
use candid::{CandidType, Principal, encode_args};
use serde::{Deserialize, Serialize};

///
/// Response
/// the root canister currently is the only one with the response() endpoint
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CanisterCreate(Principal),
    CanisterUpgrade,
}

// response
pub async fn response(req: Request) -> Result<Response, Error> {
    match req {
        Request::CanisterCreate(req) => canister_create(&req.kind, &req.parents, req.extra).await,
        Request::CanisterUpgrade(req) => canister_upgrade(req.pid, &req.kind).await,
    }
}

// canister_create
async fn canister_create(
    kind: &str,
    parents: &[CanisterParent],
    extra: Option<Vec<u8>>,
) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(kind)?;

    // only allow non-indexable canisters
    if canister.attributes.indexable {
        return Err(InterfaceError::CannotCreateIndexable)?;
    }

    // controllers are :
    // - the controllers that are specified in the config file
    // - root
    let mut controllers = Config::get()?.controllers;
    let root_pid = CanisterState::get_root_pid();
    controllers.push(root_pid);

    // encode the standard init args
    let args = encode_args((parents, extra))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // create the canister
    let new_canister_id = ic_create_canister(kind, canister.wasm, controllers, args).await?;

    Ok(Response::CanisterCreate(new_canister_id))
}

// canister_upgrade
async fn canister_upgrade(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(path)?;
    ic_upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::CanisterUpgrade)
}
