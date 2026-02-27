use crate::dto::{
    auth::{DelegationProvisionResponse, DelegationRequest},
    prelude::*,
};

///
/// Request
/// Root-directed orchestration commands.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
}

///
/// RootCapabilityRequest
/// DTO-facing capability envelope used by root dispatch internals.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum RootCapabilityRequest {
    ProvisionCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    MintCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
}

impl From<Request> for RootCapabilityRequest {
    fn from(value: Request) -> Self {
        match value {
            Request::CreateCanister(req) => Self::ProvisionCanister(req),
            Request::UpgradeCanister(req) => Self::UpgradeCanister(req),
            Request::Cycles(req) => Self::MintCycles(req),
            Request::IssueDelegation(req) => Self::IssueDelegation(req),
        }
    }
}

///
/// RootRequestMetadata
/// Replay and idempotency metadata for mutating root requests.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_seconds: u64,
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum CreateCanisterParent {
    Root,
    /// Use the requesting canister as parent.
    ThisCanister,
    /// Use the requesting canister's parent (creates a sibling).
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

///
/// UpgradeCanisterRequest
/// Payload for [`Request::UpgradeCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesRequest {
    pub cycles: u128,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// Response
/// Response payloads produced by root for orchestration requests.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
    DelegationIssued(DelegationProvisionResponse),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}
