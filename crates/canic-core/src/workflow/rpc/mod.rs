pub mod adapter;
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

    #[error("missing replay metadata for capability '{0}'")]
    MissingReplayMetadata(&'static str),

    #[error("invalid replay ttl_seconds={ttl_seconds}; max={max_ttl_seconds}")]
    InvalidReplayTtl {
        ttl_seconds: u64,
        max_ttl_seconds: u64,
    },

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

    #[error("delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml")]
    DelegatedTokensDisabled,

    #[error("delegation request must target root")]
    DelegationMustTargetRoot,

    #[error("delegation request caller {0} must match shard_pid {1}")]
    DelegationCallerShardMismatch(Principal, Principal),

    #[error("delegation ttl_secs must be greater than zero (got {0})")]
    DelegationInvalidTtl(u64),

    #[error("delegation audience must not be empty")]
    DelegationAudienceEmpty,

    #[error("delegation scopes must not be empty")]
    DelegationScopesEmpty,

    #[error("delegation scope values must not contain empty strings")]
    DelegationScopeEmpty,

    #[error(
        "delegation expires_at must be greater than issued_at (issued_at={issued_at}, expires_at={expires_at})"
    )]
    DelegationInvalidWindow { issued_at: u64, expires_at: u64 },

    #[error("delegation root pid mismatch: cert={0}, expected={1}")]
    DelegationRootPidMismatch(Principal, Principal),

    #[error("delegation shard_pid must not equal root pid")]
    DelegationShardCannotBeRoot,

    #[error("role attestation subject {subject} must match caller {caller}")]
    RoleAttestationSubjectMismatch {
        caller: Principal,
        subject: Principal,
    },

    #[error("role attestation subject {subject} is not registered in subnet registry")]
    RoleAttestationSubjectNotRegistered { subject: Principal },

    #[error(
        "role attestation role mismatch for subject {subject}: requested {requested}, registered {registered}"
    )]
    RoleAttestationRoleMismatch {
        subject: Principal,
        requested: CanisterRole,
        registered: CanisterRole,
    },

    #[error(
        "role attestation subnet mismatch for subject {subject}: requested {requested}, local {local}"
    )]
    RoleAttestationSubnetMismatch {
        subject: Principal,
        requested: Principal,
        local: Principal,
    },

    #[error("role attestation audience is required for inter-service authorization")]
    RoleAttestationAudienceRequired,

    #[error(
        "role attestation ttl_secs must satisfy 0 < ttl_secs <= {max_ttl_secs} (got {ttl_secs})"
    )]
    RoleAttestationInvalidTtl { ttl_secs: u64, max_ttl_secs: u64 },
}

impl From<RpcWorkflowError> for InternalError {
    fn from(err: RpcWorkflowError) -> Self {
        match err {
            RpcWorkflowError::CyclesFundingDisabled => {
                Self::public(PublicError::unavailable("cycles funding disabled"))
            }
            RpcWorkflowError::FundingRequestExceedsChildBudget { .. }
            | RpcWorkflowError::FundingCooldownActive { .. } => Self::public(PublicError::policy(
                ErrorCode::ResourceExhausted,
                err.to_string(),
            )),
            other => Self::workflow(InternalErrorOrigin::Workflow, other.to_string()),
        }
    }
}

///
/// TESTS
///

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
}
