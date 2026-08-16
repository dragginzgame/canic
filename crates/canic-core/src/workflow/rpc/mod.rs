//! Module: workflow::rpc
//!
//! Responsibility: define RPC workflow boundaries and shared workflow errors.
//! Does not own: endpoint DTOs, stable records, or low-level IC calls.
//! Boundary: exposes request and capability workflow modules to endpoints.

mod authority;
pub mod capability;
mod lifecycle;
pub mod request;

use crate::{InternalError, cdk::types::Principal, diagnostics::codes, ids::CanisterRole};
use thiserror::Error as ThisError;

pub use authority::{
    RootCapabilityAuthority, RootCapabilityCallerAuthority, RootCapabilityMemberAuthority,
    RootCapabilityParentAuthority,
};
pub use lifecycle::{
    RootCapabilityLifecycleExecutor, RootComponentChildProvisionRequest,
    RootComponentChildRecycleOutcome, RootComponentChildRecycleRequest,
};

///
/// RpcWorkflowError
///
/// Typed workflow failures raised while preparing or executing RPC flows.
///

#[derive(Debug, ThisError)]
pub enum RpcWorkflowError {
    #[error("canister role {0} not found")]
    CanisterRoleNotFound(CanisterRole),

    #[error("child canister {0} not found")]
    ChildNotFound(Principal),

    #[error("canister {0} is not a child of caller {1}")]
    NotChildOfCaller(Principal, Principal),

    #[error("canister {0}'s parent was not found")]
    ParentNotFound(Principal),

    #[error("insufficient funding cycles: requested={requested}, available={available}")]
    InsufficientFundingCycles { requested: u128, available: u128 },

    #[error("cycles funding disabled")]
    CyclesFundingDisabled,

    #[error(
        "funding request exceeds child budget: requested={requested}, remaining_budget={remaining_budget}, max_per_child={max_per_child}"
    )]
    FundingRequestExceedsChildBudget {
        requested: u128,
        remaining_budget: u128,
        max_per_child: u128,
    },

    #[error("funding request is in cooldown: retry_after_secs={retry_after_secs}")]
    FundingCooldownActive { retry_after_secs: u64 },

    #[error("cycles funding operation already in progress for child {child}")]
    FundingOperationInProgress { child: Principal },

    #[error("missing replay metadata for capability '{0}'")]
    MissingReplayMetadata(&'static str),

    #[error("invalid replay ttl_ns={ttl_ns}; max={max_ttl_ns}")]
    InvalidReplayTtl { ttl_ns: u64, max_ttl_ns: u64 },

    #[error("replay ttl_ns overflow: now_ns={now_ns}, ttl_ns={ttl_ns}")]
    ReplayTtlOverflow { now_ns: u64, ttl_ns: u64 },

    #[error("replay request expired for capability '{0}'")]
    ReplayExpired(&'static str),

    #[error("replay conflict for capability '{0}': request_id reused with different payload")]
    ReplayConflict(&'static str),

    #[error(
        "duplicate replay request for capability '{0}': request_id reused with identical payload"
    )]
    ReplayDuplicateSame(&'static str),

    #[error("replay cache encode failed: {0}")]
    ReplayEncodeFailed(String),

    #[error("replay cache decode failed: {0}")]
    ReplayDecodeFailed(String),

    #[error("replay store capacity reached ({0})")]
    ReplayStoreCapacityReached(usize),

    #[error("replay store caller capacity reached for {caller} ({max_entries})")]
    ReplayStoreCallerCapacityReached {
        caller: Principal,
        max_entries: usize,
    },
}

impl From<RpcWorkflowError> for InternalError {
    fn from(err: RpcWorkflowError) -> Self {
        match err {
            RpcWorkflowError::CanisterRoleNotFound(_)
            | RpcWorkflowError::ChildNotFound(_)
            | RpcWorkflowError::ParentNotFound(_) => Self::public(codes::AUTHORITY_UNAVAILABLE),
            RpcWorkflowError::NotChildOfCaller(_, _) => {
                Self::public(codes::AUTHORITY_INVALID_STATE)
            }
            RpcWorkflowError::InsufficientFundingCycles { .. } => {
                Self::public(codes::CAPACITY_INSUFFICIENT)
            }
            RpcWorkflowError::CyclesFundingDisabled => Self::public(codes::CAPACITY_INACTIVE),
            RpcWorkflowError::MissingReplayMetadata(_) => {
                Self::public(codes::AUTHORITY_UNAVAILABLE)
            }
            RpcWorkflowError::FundingRequestExceedsChildBudget { .. }
            | RpcWorkflowError::ReplayStoreCapacityReached(_) => {
                Self::public(codes::CAPACITY_LIMIT)
            }
            RpcWorkflowError::FundingCooldownActive { .. } => {
                Self::public(codes::CAPACITY_UNEXPECTED_STATE)
            }
            RpcWorkflowError::FundingOperationInProgress { .. }
            | RpcWorkflowError::ReplayDuplicateSame(_) => Self::public(codes::REQUEST_INCOMPLETE),
            RpcWorkflowError::InvalidReplayTtl { ttl_ns: 0, .. } => {
                Self::public(codes::TIME_INVALID)
            }
            RpcWorkflowError::InvalidReplayTtl { .. } => Self::public(codes::TIME_CAPACITY),
            RpcWorkflowError::ReplayTtlOverflow { .. } => Self::public(codes::CAPACITY_UNSUPPORTED),
            RpcWorkflowError::ReplayExpired(_) => Self::public(codes::EVIDENCE_EXPIRED),
            RpcWorkflowError::ReplayConflict(_) => Self::public(codes::CODEC_CONFLICT),
            RpcWorkflowError::ReplayEncodeFailed(_) | RpcWorkflowError::ReplayDecodeFailed(_) => {
                Self::public(codes::CODEC_FAILED)
            }
            RpcWorkflowError::ReplayStoreCallerCapacityReached { .. } => {
                Self::public(codes::AUTHORITY_CAPACITY)
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_funding_disabled_maps_to_unavailable_public_error() {
        let internal: InternalError = RpcWorkflowError::CyclesFundingDisabled.into();
        let public = internal.public_error();
        assert_eq!(public.code(), codes::CAPACITY_INACTIVE.raw_code());
    }

    #[test]
    fn missing_replay_metadata_maps_to_operation_id_required() {
        let internal: InternalError =
            RpcWorkflowError::MissingReplayMetadata("RequestCycles").into();
        let public = internal.public_error();

        assert_eq!(public.code(), codes::AUTHORITY_UNAVAILABLE.raw_code());
    }

    #[test]
    fn insufficient_funding_cycles_preserves_resource_exhaustion_cause() {
        let internal: InternalError = RpcWorkflowError::InsufficientFundingCycles {
            requested: 5_000,
            available: 4_000,
        }
        .into();
        let public = internal.public_error();

        assert_eq!(public.code(), codes::CAPACITY_INSUFFICIENT.raw_code());
    }
}
