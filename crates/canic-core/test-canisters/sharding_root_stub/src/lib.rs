//!
//! Minimal root stub for PocketIC sharding tests.
//!

use canic::{
    Error, cdk,
    dto::rpc::{
        CreateCanisterResponse, CyclesResponse, Request, Response, UpgradeCanisterResponse,
    },
};

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;

#[cdk::init]
fn init() {}

#[cdk::update]
async fn canic_response(request: Request) -> Result<Response, Error> {
    match request {
        Request::CreateCanister(_) => {
            let pid = create_canister().await?;
            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid: pid,
            }))
        }
        Request::UpgradeCanister(_) => Ok(Response::UpgradeCanister(UpgradeCanisterResponse {})),
        Request::Cycles(req) => Ok(Response::Cycles(CyclesResponse {
            cycles_transferred: req.cycles,
        })),
    }
}

async fn create_canister() -> Result<cdk::types::Principal, Error> {
    let args = cdk::mgmt::CreateCanisterArgs { settings: None };

    let res = cdk::mgmt::create_canister_with_extra_cycles(&args, CREATE_CANISTER_CYCLES)
        .await
        .map_err(|err| Error::internal(format!("create_canister failed: {err}")))?;

    Ok(res.canister_id)
}

cdk::export_candid!();
