//! Module: managed child Root fixture
//!
//! Responsibility: provide a replay-safe Root allocation peer for public managed-tree tests.
//! Does not own: production Component Registry state, installation, or placement policy.
//! Boundary: host settlement installs children and synchronizes their parent before replies.

mod codec;

use canic::{
    Error,
    dto::capability::RootCapabilityResponseV1,
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
        CreateCanisterResponse, CyclesResponse, Request, Response,
    },
    ids::CanisterRole,
};
use canic_contracts::dto::wire::projection::capability_rpc::{
    RootCommandFragment, RootCommandResponseFragment,
};
use ic_platform::cdk::call::Call;
use ic_platform::{
    candid::contract::{CandidType, DecoderConfig, Nat, decode_args_with_config, encode_one},
    principal::ExternalPrincipal as Principal,
};
use serde::Deserialize;
use std::cell::RefCell;

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;
const MAX_SETTLEMENT_POLLS: usize = 4_096;

thread_local! {
    static ALLOCATIONS: RefCell<Vec<FixtureChildAllocation>> = const { RefCell::new(Vec::new()) };
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
struct FixtureChildAllocation {
    request_id: [u8; 32],
    parent: Principal,
    canister_role: CanisterRole,
    extra_arg: Option<Vec<u8>>,
    child: Principal,
    settled: bool,
    acknowledged: bool,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
struct StubCreateCanisterArgs {
    settings: Option<StubCanisterSettings>,
    sender_canister_version: Option<u64>,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
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
#[candid_path("ic_platform::candid::contract")]
enum StubLogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
struct StubEnvironmentVariable {
    name: String,
    value: String,
}

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
struct StubCreateCanisterResult {
    canister_id: Principal,
}

#[ic_platform::cdk::init]
const fn init() {}

#[ic_platform::cdk::update(decode_with = "codec::decode_one", encode_with = "codec::encode_reply")]
async fn canic_root_command(
    command: RootCommandFragment,
) -> Result<RootCommandResponseFragment, Error> {
    let RootCommandFragment::RespondCapability(envelope) = command;
    let request_id = envelope.metadata.request_id;
    let response = RootCapabilityResponseV1 {
        response: handle_request(request_id, envelope.capability).await?,
    };
    Ok(RootCommandResponseFragment::RespondCapability(response))
}

#[ic_platform::cdk::query(
    decode_with = "codec::decode_args",
    encode_with = "codec::encode_reply"
)]
fn testing_component_child_allocations() -> Vec<FixtureChildAllocation> {
    ALLOCATIONS.with_borrow(Clone::clone)
}

#[ic_platform::cdk::query(decode_with = "codec::decode_one", encode_with = "codec::encode_reply")]
fn testing_component_child_is_settled(request_id: [u8; 32]) -> bool {
    ALLOCATIONS.with_borrow(|allocations| {
        allocations
            .iter()
            .any(|allocation| allocation.request_id == request_id && allocation.settled)
    })
}

#[ic_platform::cdk::update(decode_with = "codec::decode_one", encode_with = "codec::encode_reply")]
fn testing_settle_component_child(request_id: [u8; 32]) -> Result<(), Error> {
    ALLOCATIONS.with_borrow_mut(|allocations| {
        let allocation = allocations
            .iter_mut()
            .find(|allocation| allocation.request_id == request_id)
            .ok_or_else(|| Error::from_registered(canic::diagnostics::codes::STATE_CONFLICT))?;
        allocation.settled = true;
        Ok(())
    })
}

async fn handle_request(request_id: [u8; 32], request: Request) -> Result<Response, Error> {
    match request {
        Request::AcknowledgePlacementReceipt(request) => acknowledge(request_id, &request),
        Request::AllocatePlacementChild(request) | Request::CreateCanister(request) => {
            let pid = allocate_child(request_id, request).await?;
            await_settlement(request_id).await?;
            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid: pid,
            }))
        }
        Request::RecycleCanister(_) => Ok(Response::RecycleCanister),
        Request::Cycles(req) => Ok(Response::Cycles(CyclesResponse::Transferred {
            cycles_transferred: req.cycles,
        })),
    }
}

async fn allocate_child(
    request_id: [u8; 32],
    request: CreateCanisterRequest,
) -> Result<Principal, Error> {
    let parent =
        ic_platform::principal::ExternalPrincipal::from(ic_platform::cdk::api::msg_caller());
    if request_id == [0; 32]
        || request.metadata.map(|metadata| metadata.request_id) != Some(request_id)
        || !matches!(request.parent, CreateCanisterParent::ThisCanister)
    {
        return Err(Error::from_registered(
            canic::diagnostics::codes::REQUEST_INVALID,
        ));
    }

    if let Some(existing) = ALLOCATIONS.with_borrow(|allocations| {
        allocations
            .iter()
            .find(|allocation| allocation.request_id == request_id)
            .cloned()
    }) {
        if existing.parent != parent
            || existing.canister_role != request.canister_role
            || existing.extra_arg != request.extra_arg
        {
            return Err(Error::from_registered(
                canic::diagnostics::codes::STATE_CONFLICT,
            ));
        }
        return Ok(existing.child);
    }

    let child = create_canister().await?;
    ALLOCATIONS.with_borrow_mut(|allocations| {
        allocations.push(FixtureChildAllocation {
            request_id,
            parent,
            canister_role: request.canister_role,
            extra_arg: request.extra_arg,
            child,
            settled: false,
            acknowledged: false,
        });
    });
    Ok(child)
}

// Yield through actual IC calls while the host installs and publishes the child.
// Returning before settlement would violate the production allocation protocol.
async fn await_settlement(request_id: [u8; 32]) -> Result<(), Error> {
    for _ in 0..MAX_SETTLEMENT_POLLS {
        let response = Call::bounded_wait(
            ic_platform::cdk::api::canister_self(),
            "testing_component_child_is_settled",
        )
        .take_raw_args(
            encode_one(request_id).unwrap_or_else(|error| panic!("failed to encode args: {error}")),
        )
        .await
        .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;
        let (settled,): (bool,) = decode_args_with_config(response.as_ref(), &DecoderConfig::new())
            .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;
        if settled {
            return Ok(());
        }
    }
    Err(Error::from_registered(
        canic::diagnostics::codes::STATE_UNAVAILABLE,
    ))
}

fn acknowledge(
    request_id: [u8; 32],
    request: &AcknowledgePlacementReceiptRequest,
) -> Result<Response, Error> {
    if request_id == [0; 32]
        || request.operation_id != request_id
        || request.metadata.map(|metadata| metadata.request_id) != Some(request_id)
    {
        return Err(Error::from_registered(
            canic::diagnostics::codes::REQUEST_INVALID,
        ));
    }
    let acknowledged = ALLOCATIONS.with_borrow_mut(|allocations| {
        let Some(allocation) = allocations
            .iter_mut()
            .find(|allocation| allocation.request_id == request_id)
        else {
            return false;
        };
        if !allocation.settled {
            return false;
        }
        allocation.acknowledged = true;
        true
    });
    if !acknowledged {
        return Err(Error::from_registered(
            canic::diagnostics::codes::STATE_CONFLICT,
        ));
    }
    Ok(Response::AcknowledgePlacementReceipt)
}

async fn create_canister() -> Result<Principal, Error> {
    let args = StubCreateCanisterArgs {
        settings: None,
        sender_canister_version: Some(ic_platform::cdk::api::canister_version()),
    };

    let response = Call::bounded_wait(Principal::management_canister().into(), "create_canister")
        .take_raw_args(
            encode_one(args).unwrap_or_else(|error| panic!("failed to encode args: {error}")),
        )
        .with_cycles(CREATE_CANISTER_CYCLES)
        .await
        .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;
    let (res,): (StubCreateCanisterResult,) =
        decode_args_with_config(response.as_ref(), &DecoderConfig::new())
            .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;

    Ok(res.canister_id)
}

canic::finish!();
