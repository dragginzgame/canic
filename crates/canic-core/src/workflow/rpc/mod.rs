//! Module: workflow::rpc
//!
//! Responsibility: define RPC workflow boundaries and shared workflow errors.
//! Does not own: endpoint DTOs, stable records, or low-level IC calls.
//! Boundary: exposes request and capability workflow modules to endpoints.

pub mod capability;
pub mod request;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::error::{Error as PublicError, ErrorCode},
    ids::CanisterRole,
};
use thiserror::Error as ThisError;

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

    #[error("create_canister: missing new pid")]
    MissingNewCanisterPid,

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
            RpcWorkflowError::CyclesFundingDisabled => {
                Self::public(PublicError::unavailable("cycles funding disabled"))
            }
            RpcWorkflowError::MissingReplayMetadata(_) => {
                Self::public(PublicError::operation_id_required())
            }
            RpcWorkflowError::InsufficientFundingCycles { .. }
            | RpcWorkflowError::FundingRequestExceedsChildBudget { .. }
            | RpcWorkflowError::FundingCooldownActive { .. } => Self::public(PublicError::policy(
                ErrorCode::ResourceExhausted,
                err.to_string(),
            )),
            RpcWorkflowError::FundingOperationInProgress { .. } => {
                Self::public(PublicError::conflict(err.to_string()))
            }
            other => Self::workflow(InternalErrorOrigin::Workflow, other.to_string()),
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::error::ErrorCode;

    #[test]
    fn cycles_funding_disabled_maps_to_unavailable_public_error() {
        let internal: InternalError = RpcWorkflowError::CyclesFundingDisabled.into();
        let public = internal
            .public_error()
            .expect("expected public error mapping for kill switch");
        assert_eq!(public.code, ErrorCode::Unavailable);
    }

    #[test]
    fn missing_replay_metadata_maps_to_operation_id_required() {
        let internal: InternalError =
            RpcWorkflowError::MissingReplayMetadata("RequestCycles").into();
        let public = internal
            .public_error()
            .expect("expected public replay metadata error");

        assert_eq!(public.code, ErrorCode::OperationIdRequired);
    }

    #[test]
    fn insufficient_funding_cycles_preserves_resource_exhaustion_cause() {
        let internal: InternalError = RpcWorkflowError::InsufficientFundingCycles {
            requested: 5_000,
            available: 4_000,
        }
        .into();
        let public = internal
            .public_error()
            .expect("expected public funding-capacity error");

        assert_eq!(public.code, ErrorCode::ResourceExhausted);
    }
}
