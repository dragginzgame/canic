use crate::dto::prelude::*;

//
// Request
//
// Root orchestration request.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest),
    AllocatePlacementChild(CreateCanisterRequest),
    CreateCanister(CreateCanisterRequest),
    RecycleCanister(RecycleCanisterRequest),
    Cycles(CyclesRequest),
}

impl Request {
    // acknowledge_placement_receipt
    //
    // Build a root request that releases one durably consumed placement receipt.
    #[must_use]
    pub const fn acknowledge_placement_receipt(
        request: AcknowledgePlacementReceiptRequest,
    ) -> Self {
        Self::AcknowledgePlacementReceipt(request)
    }

    // allocate_placement_child
    //
    // Build a retained root request for receipt-backed placement allocation.
    #[must_use]
    pub const fn allocate_placement_child(request: CreateCanisterRequest) -> Self {
        Self::AllocatePlacementChild(request)
    }

    // create_canister
    //
    // Build a root request for canister provisioning.
    #[must_use]
    pub const fn create_canister(request: CreateCanisterRequest) -> Self {
        Self::CreateCanister(request)
    }

    // recycle_canister
    //
    // Build a root request for recycling one child canister back into the pool.
    #[must_use]
    pub const fn recycle_canister(request: RecycleCanisterRequest) -> Self {
        Self::RecycleCanister(request)
    }

    // cycles
    //
    // Build a root request for requesting/transferring cycles.
    #[must_use]
    pub const fn cycles(request: CyclesRequest) -> Self {
        Self::Cycles(request)
    }
}

//
// AcknowledgePlacementReceiptRequest
//
// Placement-receipt acknowledgement payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct AcknowledgePlacementReceiptRequest {
    pub operation_id: [u8; 32],
    pub metadata: Option<RootRequestMetadata>,
}

//
// RootRequestMetadata
//
// Replay metadata.
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_ns: u64,
}

//
// CreateCanisterRequest
//
// Create-canister payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
    pub metadata: Option<RootRequestMetadata>,
}

//
// CreateCanisterParent
//
// Parent selection.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CreateCanisterParent {
    Root,
    // Use the requesting canister.
    ThisCanister,
    // Use the caller's parent.
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

//
// RecycleCanisterRequest
//
// Recycle-one-child payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RecycleCanisterRequest {
    pub canister_pid: Principal,
    pub metadata: Option<RootRequestMetadata>,
}

//
// CyclesRequest
//
// Cycles payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
    pub metadata: Option<RootRequestMetadata>,
}

//
// Response
//
// Root response payload.
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    AcknowledgePlacementReceipt,
    CreateCanister(CreateCanisterResponse),
    RecycleCanister,
    Cycles(CyclesResponse),
}

//
// CreateCanisterResponse
// Result of creating and installing a new canister.
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

//
// CyclesFundingPreflightResponse
// Typed caller-owned context for a request rejected before any cycles transfer.
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CyclesFundingPreflightResponse {
    ChildBudgetExhausted {
        remaining_child_budget: u128,
        max_per_child: u128,
    },
    CooldownActive {
        retry_after_secs: u64,
    },
    ParentFundingUnavailable {
        approved_cycles: u128,
    },
}

//
// CyclesResponse
// Typed outcome of requesting cycles from the current parent.
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CyclesResponse {
    PreflightRejected(CyclesFundingPreflightResponse),
    Transferred { cycles_transferred: u128 },
}

impl CyclesResponse {
    /// Return the transferred amount only for a completed funding effect.
    #[must_use]
    pub const fn cycles_transferred(self) -> Option<u128> {
        match self {
            Self::Transferred { cycles_transferred } => Some(cycles_transferred),
            Self::PreflightRejected(_) => None,
        }
    }
}
