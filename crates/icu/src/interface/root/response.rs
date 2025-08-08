use crate::{
    Error,
    interface::{
        InterfaceError,
        cascade::subnet_index_cascade,
        ic::{IcError, ic_create_canister, ic_upgrade_canister},
        request::Request,
        root::new_canister_controllers,
    },
    memory::canister::CanisterParent,
    state::CanisterRegistry,
};
use candid::{CandidType, Principal, encode_args};
use serde::{Deserialize, Serialize};

///
/// Response
/// the root canister is the only one with the response() endpoint
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CreateCanister(Principal),
    UpgradeCanister,
}

// response
pub async fn response(req: Request) -> Result<Response, Error> {
    match req {
        Request::CreateCanister(req) => {
            response_create_canister(&req.kind, &req.parents, req.extra).await
        }
        Request::UpgradeCanister(req) => response_upgrade_canister(req.pid, &req.kind).await,
    }
}

// response_create_canister
async fn response_create_canister(
    kind: &str,
    parents: &[CanisterParent],
    extra: Option<Vec<u8>>,
) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(kind)?;

    // only allow non-indexable canisters
    if canister.attributes.indexable {
        return Err(InterfaceError::CannotCreateIndexable)?;
    }

    // encode the standard init args
    let args = encode_args((parents, extra))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // create the canister
    let controllers = new_canister_controllers()?;
    let new_canister_id = ic_create_canister(kind, canister.wasm, controllers, args).await?;

    // cascade subnet, as we're on root
    subnet_index_cascade().await?;

    Ok(Response::CreateCanister(new_canister_id))
}

// response_upgrade_canister
async fn response_upgrade_canister(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(path)?;
    ic_upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::UpgradeCanister)
}
