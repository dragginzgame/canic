use crate::{
    Error,
    ic::api::{canister_cycle_balance, msg_caller},
    interface::{
        ic::{create_and_install_canister, deposit_cycles, upgrade_canister},
        request::{CreateCanisterRequest, CyclesRequest, Request, UpgradeCanisterRequest},
    },
    memory::CanisterState,
    state::canister::CanisterCatalog,
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
    let new_canister_pid =
        create_and_install_canister(&req.canister_type, &req.parents, req.extra_arg.clone())
            .await?;

    Ok(Response::CreateCanister(CreateCanisterResponse {
        new_canister_pid,
    }))
}

// upgrade_canister_response
async fn upgrade_canister_response(req: &UpgradeCanisterRequest) -> Result<Response, Error> {
    let canister = CanisterCatalog::try_get(&req.canister_type)?;
    upgrade_canister(req.canister_pid, canister.wasm).await?;

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
