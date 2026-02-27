pub mod adapter;
pub mod request;

use crate::{InternalError, InternalErrorOrigin, cdk::types::Principal, ids::CanisterRole};
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

    #[error("insufficient root cycles: requested={requested}, available={available}")]
    InsufficientRootCycles { requested: u128, available: u128 },

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
}

impl From<RpcWorkflowError> for InternalError {
    fn from(err: RpcWorkflowError) -> Self {
        Self::workflow(InternalErrorOrigin::Workflow, err.to_string())
    }
}
