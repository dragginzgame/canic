use crate::dto::prelude::*;

///
/// Request
/// Root-directed orchestration commands.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
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

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
}

///
/// Response
/// Response payloads produced by root for orchestration requests.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}
