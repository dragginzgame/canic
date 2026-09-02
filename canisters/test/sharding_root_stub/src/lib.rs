//! Module: managed child Root fixture
//!
//! Responsibility: provide a replay-safe Root allocation peer for public managed-tree tests.
//! Does not own: production Component Registry state, installation, or placement policy.
//! Boundary: creates empty PocketIC children and exposes their exact retained request identity.

use candid::{CandidType, Deserialize, Nat, Principal};
use canic::{
    Error,
    dto::capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
        CreateCanisterResponse, CyclesResponse, Request, Response,
    },
    ids::CanisterRole,
};
use ic_cdk::call::Call;
use std::cell::RefCell;

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;

thread_local! {
    static ALLOCATIONS: RefCell<Vec<FixtureChildAllocation>> = const { RefCell::new(Vec::new()) };
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct FixtureChildAllocation {
    request_id: [u8; 32],
    parent: Principal,
    canister_role: CanisterRole,
    extra_arg: Option<Vec<u8>>,
    child: Principal,
    acknowledged: bool,
}

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
const fn init() {}

#[derive(CandidType, Deserialize)]
enum RootCommand {
    RespondCapability(RootCapabilityEnvelopeV1),
}

#[derive(CandidType)]
enum RootCommandResponse {
    RespondCapability(RootCapabilityResponseV1),
}

#[ic_cdk::update]
async fn canic_root_command(command: RootCommand) -> Result<RootCommandResponse, Error> {
    let RootCommand::RespondCapability(envelope) = command;
    let request_id = envelope.metadata.request_id;
    let response = RootCapabilityResponseV1 {
        response: handle_request(request_id, envelope.capability).await?,
    };
    Ok(RootCommandResponse::RespondCapability(response))
}

#[ic_cdk::query]
fn testing_component_child_allocations() -> Vec<FixtureChildAllocation> {
    ALLOCATIONS.with_borrow(Clone::clone)
}

async fn handle_request(request_id: [u8; 32], request: Request) -> Result<Response, Error> {
    match request {
        Request::AcknowledgePlacementReceipt(request) => acknowledge(request_id, &request),
        Request::AllocatePlacementChild(request) | Request::CreateCanister(request) => {
            let pid = allocate_child(request_id, request).await?;
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
    let parent = ic_cdk::api::msg_caller();
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
            acknowledged: false,
        });
    });
    Ok(child)
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
        sender_canister_version: Some(ic_cdk::api::canister_version()),
    };

    let response = Call::bounded_wait(Principal::management_canister(), "create_canister")
        .with_arg(args)
        .with_cycles(CREATE_CANISTER_CYCLES)
        .await
        .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;
    let res: StubCreateCanisterResult = response
        .candid()
        .map_err(|_| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;

    Ok(res.canister_id)
}

canic::finish!();
