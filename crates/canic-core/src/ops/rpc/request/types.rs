/// Request
///
/// Ops-local alias for the canonical RPC request DTO.
pub type Request = crate::dto::rpc::Request;

/// RootRequestMetadata
///
/// Ops-local alias for replay/idempotency request metadata.
pub type RootRequestMetadata = crate::dto::rpc::RootRequestMetadata;

/// CreateCanisterRequest
///
/// Ops-local alias for canister-provision request payload.
pub type CreateCanisterRequest = crate::dto::rpc::CreateCanisterRequest;

/// CreateCanisterParent
///
/// Ops-local alias for parent-placement selector.
pub type CreateCanisterParent = crate::dto::rpc::CreateCanisterParent;

/// UpgradeCanisterRequest
///
/// Ops-local alias for canister-upgrade request payload.
pub type UpgradeCanisterRequest = crate::dto::rpc::UpgradeCanisterRequest;

/// CyclesRequest
///
/// Ops-local alias for cycles request payload.
pub type CyclesRequest = crate::dto::rpc::CyclesRequest;

/// Response
///
/// Ops-local alias for canonical RPC response DTO.
pub type Response = crate::dto::rpc::Response;

/// CreateCanisterResponse
///
/// Ops-local alias for canister-provision response payload.
pub type CreateCanisterResponse = crate::dto::rpc::CreateCanisterResponse;

/// UpgradeCanisterResponse
///
/// Ops-local alias for canister-upgrade response payload.
pub type UpgradeCanisterResponse = crate::dto::rpc::UpgradeCanisterResponse;

/// CyclesResponse
///
/// Ops-local alias for cycles response payload.
pub type CyclesResponse = crate::dto::rpc::CyclesResponse;
