//!
//! Minimal root stub for PocketIC sharding tests.
//!

use candid::{CandidType, Deserialize, Nat, Principal};
use canic::{
    Error,
    dto::capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
    dto::rpc::{CreateCanisterResponse, CyclesResponse, Request, Response},
};
use ic_cdk::call::Call;

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;

#[derive(CandidType)]
struct StubCreateCanisterArgs {
    settings: Option<StubCanisterSettings>,
    sender_canister_version: Option<u64>,
}

#[derive(CandidType)]
struct StubCanisterSettings {
    controllers: Option<Vec<Principal>>,
    compute_allocation: Option<Nat>,
    memory_allocation: Option<Nat>,
    freezing_threshold: Option<Nat>,
    reserved_cycles_limit: Option<Nat>,
    log_visibility: Option<StubLogVisibility>,
    log_memory_limit: Option<Nat>,
    wasm_memory_limit: Option<Nat>,
    wasm_memory_threshold: Option<Nat>,
    environment_variables: Option<Vec<StubEnvironmentVariable>>,
}

#[derive(CandidType, Deserialize)]
enum StubLogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

#[derive(CandidType)]
struct StubEnvironmentVariable {
    name: String,
    value: String,
}

#[derive(CandidType, Deserialize)]
struct StubCreateCanisterResult {
    canister_id: Principal,
}

#[ic_cdk::init]
fn init() {}

#[ic_cdk::update]
async fn canic_response_capability_v1(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let response = handle_request(envelope.capability).await?;
    Ok(RootCapabilityResponseV1 { response })
}

async fn handle_request(request: Request) -> Result<Response, Error> {
    match request {
        Request::AcknowledgePlacementReceipt(_) => Ok(Response::AcknowledgePlacementReceipt),
        Request::AllocatePlacementChild(_) | Request::CreateCanister(_) => {
            let pid = create_canister().await?;
            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid: pid,
            }))
        }
        Request::UpgradeCanister(_) => Ok(Response::UpgradeCanister),
        Request::RecycleCanister(_) => Ok(Response::RecycleCanister),
        Request::Cycles(req) => Ok(Response::Cycles(CyclesResponse {
            cycles_transferred: req.cycles,
        })),
    }
}

async fn create_canister() -> Result<Principal, Error> {
    let args = StubCreateCanisterArgs {
        settings: None,
        sender_canister_version: Some(ic_cdk::api::canister_version()),
    };

    let response = Call::bounded_wait(Principal::management_canister(), "create_canister")
        .with_arg(args)
        .with_cycles(CREATE_CANISTER_CYCLES)
        .await
        .map_err(|err| Error::internal(format!("create_canister failed: {err}")))?;
    let res: StubCreateCanisterResult = response
        .candid()
        .map_err(|err| Error::internal(format!("create_canister response failed: {err}")))?;

    Ok(res.canister_id)
}

canic::finish!();
