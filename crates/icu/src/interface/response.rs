use crate::{
    Error,
    config::Config,
    interface::{
        self, InterfaceError,
        ic::{IcError, ic_create_canister, ic_upgrade_canister},
        request::Request,
    },
    memory::{CanisterState, SubnetIndex, canister::CanisterParent},
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
        Request::CanisterCreate(req) => {
            root_canister_create(&req.kind, &req.parents, req.extra).await
        }
        Request::CanisterUpgrade(req) => root_canister_upgrade(req.pid, &req.kind).await,
    }
}

// root_canister_create
async fn root_canister_create(
    kind: &str,
    parents: &[CanisterParent],
    extra: Option<Vec<u8>>,
) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(kind)?;
    let root_pid = CanisterState::get_root_pid();

    // controllers are :
    // - the controllers that are specified in the config file
    // - root
    let mut controllers = Config::get()?.controllers;
    controllers.push(root_pid);

    // encode the standard init args
    let args = encode_args((parents, extra))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // create the canister
    let new_canister_id = ic_create_canister(kind, canister.wasm, controllers, args).await?;

    // optional - update subnet index
    if !canister.attributes.is_sharded {
        SubnetIndex::insert(kind, new_canister_id);
        interface::cascade::subnet_index_cascade().await?;
    }

    Ok(Response::CanisterCreate(new_canister_id))
}

// root_canister_upgrade
async fn root_canister_upgrade(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(path)?;
    ic_upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::CanisterUpgrade)
}
