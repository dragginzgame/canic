use crate::{
    Error,
    ic::api::{canister_cycle_balance, msg_caller},
    interface::{
        InterfaceError,
        ic::{deposit_cycles, ic_create_canister, ic_upgrade_canister},
        request::{CreateCanisterRequest, CyclesRequest, Request, UpgradeCanisterRequest},
        state::{StateBundle, cascade, update_canister},
    },
    memory::{CanisterState, SubnetDirectory, SubnetRegistry, SubnetRegistryEntry},
    state::canister::CanisterRegistry,
};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// Response
/// the root canister is the only one with the response() endpoint
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
}

///
/// CreateCanisterResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
    pub new_balance: u128,
}

// response
pub async fn response(req: Request) -> Result<Response, Error> {
    assert!(CanisterState::is_root(), "only root can run this code");

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}

// create_canister_response
async fn create_canister_response(req: &CreateCanisterRequest) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(&req.kind)?;
    let created_under_root = req.parents.len() <= 1;
    let uses_directory = canister.attributes.uses_directory();

    // canisters have to be on root
    // and subnet_index has to be able to accept them
    if uses_directory {
        if created_under_root {
            SubnetDirectory::can_insert(&canister)?;
        } else {
            return Err(InterfaceError::CannotCreateIndexable)?;
        }
    }
    let kind = req.kind.clone();

    let new_canister_pid =
        ic_create_canister(&kind, canister.wasm, &req.parents, req.extra_arg.clone()).await?;

    // add to SubnetRegistry
    SubnetRegistry::insert(SubnetRegistryEntry {
        canister_pid: new_canister_pid,
        kind,
        parent_pid: req.parents.last().map(|p| p.principal),
    });

    // if root creates a indexable canister, cascade
    if created_under_root && uses_directory {
        SubnetDirectory::insert(&canister, new_canister_pid)?;

        // update directly as it won't yet be in the child index
        let bundle = StateBundle::subnet_index();
        update_canister(&new_canister_pid, &bundle).await?;

        // cascade to existing child index
        cascade(&bundle).await?;
    }

    Ok(Response::CreateCanister(CreateCanisterResponse {
        new_canister_pid,
    }))
}

// upgrade_canister_response
async fn upgrade_canister_response(req: &UpgradeCanisterRequest) -> Result<Response, Error> {
    let canister = CanisterRegistry::try_get(&req.kind)?;
    ic_upgrade_canister(req.canister_pid, canister.wasm).await?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    let balance = canister_cycle_balance();

    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;
    let new_balance = balance - cycles_transferred;

    Ok(Response::Cycles(CyclesResponse {
        cycles_transferred,
        new_balance,
    }))
}
