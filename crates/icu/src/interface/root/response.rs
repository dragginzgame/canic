use crate::{
    Error,
    interface::{
        InterfaceError,
        ic::{ic_create_canister, ic_upgrade_canister},
        request::Request,
        state::{StateBundle, cascade, update_canister},
    },
    memory::{CanisterState, SubnetIndex, canister::CanisterParent},
    state::CanisterRegistry,
};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// Response
/// the root canister is the only one with the response() endpoint
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Response {
    CreateCanister(Principal),
    UpgradeCanister,
}

// response
pub async fn response(req: Request) -> Result<Response, Error> {
    match req {
        Request::CreateCanister(req) => {
            create_canister_response(&req.kind, &req.parents, req.extra).await
        }
        Request::UpgradeCanister(req) => upgrade_canister_response(req.pid, &req.kind).await,
    }
}

// create_canister_response
async fn create_canister_response(
    kind: &str,
    parents: &[CanisterParent],
    extra_arg: Option<Vec<u8>>,
) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(kind)?;
    let is_root = CanisterState::is_root();

    // only allow non-indexable canisters
    if !is_root && canister.attributes.indexable {
        return Err(InterfaceError::CannotCreateIndexable)?;
    }

    // create the canister
    let new_canister_id = ic_create_canister(kind, canister.wasm, parents, extra_arg).await?;

    // if root creates a indexable canister, cascade
    if is_root && canister.attributes.indexable {
        SubnetIndex::insert(kind, new_canister_id);

        let bundle = StateBundle::subnet_index();

        // update directly as it won't yet be in the child index
        update_canister(&new_canister_id, &bundle).await?;

        // cascade to existing child index
        cascade(&bundle).await?;
    }

    Ok(Response::CreateCanister(new_canister_id))
}

// upgrade_canister_response
async fn upgrade_canister_response(pid: Principal, path: &str) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(path)?;
    ic_upgrade_canister(pid, canister.wasm).await?;

    Ok(Response::UpgradeCanister)
}
