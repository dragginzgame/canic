//!
//! Minimal root stub for PocketIC sharding tests.
//!

use canic::{
    Error, cdk,
    dto::capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
    dto::rpc::{
        CreateCanisterResponse, CyclesResponse, RecycleCanisterResponse, Request, Response,
        UpgradeCanisterResponse,
    },
};

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;

#[cdk::init]
fn init() {}

#[cdk::update]
async fn canic_response_capability_v1(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let response = handle_request(envelope.capability).await?;
    Ok(RootCapabilityResponseV1 { response })
}

async fn handle_request(request: Request) -> Result<Response, Error> {
    match request {
        Request::CreateCanister(_) => {
            let pid = create_canister().await?;
            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid: pid,
            }))
        }
        Request::UpgradeCanister(_) => Ok(Response::UpgradeCanister(UpgradeCanisterResponse {})),
        Request::RecycleCanister(_) => Ok(Response::RecycleCanister(RecycleCanisterResponse {})),
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

canic::finish!();
