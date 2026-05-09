use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::DirectoryConfig,
    ids::CanisterRole,
    ops::{
        ic::IcOps,
        runtime::metrics::directory::DirectoryMetricReason as MetricReason,
        storage::{
            children::CanisterChildrenOps, placement::directory::DirectoryRegistryOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error as ThisError;

///
/// DirectoryWorkflowError
///

#[derive(Debug, ThisError)]
pub(super) enum DirectoryWorkflowError {
    #[error("directory placement is not configured for the current canister")]
    DirectoryDisabled,

    #[error("unknown directory pool '{requested}': configured pools: {available}")]
    UnknownPool {
        requested: String,
        available: String,
    },

    #[error("instance {0} is not a direct child of the current canister")]
    InstanceNotDirectChild(Principal),

    #[error("directory instance {pid} has role '{actual}', expected '{expected}'")]
    InstanceRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        actual: CanisterRole,
    },

    #[error("directory instance {0} is not present in the subnet registry")]
    RegistryEntryMissing(Principal),
}

impl From<DirectoryWorkflowError> for InternalError {
    fn from(err: DirectoryWorkflowError) -> Self {
        Self::domain(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// DirectoryEntryClassification
///

#[derive(Debug, Eq, PartialEq)]
pub(super) enum DirectoryEntryClassification {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingFresh {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Repairable {
        claim_id: u64,
        provisional_pid: Principal,
    },
    NeedsCleanup {
        claim_id: u64,
        provisional_pid: Option<Principal>,
    },
}

static DIRECTORY_CLAIM_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn available_pool_names(directory: &DirectoryConfig) -> String {
    if directory.pools.is_empty() {
        return "none".to_string();
    }

    let mut names: Vec<_> = directory.pools.keys().cloned().collect();
    names.sort();
    names.join(", ")
}

pub(super) fn new_claim_id() -> u64 {
    let nonce = DIRECTORY_CLAIM_NONCE.fetch_add(1, Ordering::Relaxed);
    IcOps::now_millis().rotate_left(21) ^ nonce
}

pub(super) const fn pending_is_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > DirectoryRegistryOps::PENDING_TTL_SECS
}

// Validate a bind target while preserving a bounded metric reason for callers.
pub(super) fn validate_bind_target_with_reason(
    pid: Principal,
    expected_role: &CanisterRole,
) -> Result<(), (InternalError, MetricReason)> {
    if !CanisterChildrenOps::data()
        .entries
        .iter()
        .any(|(child_pid, _)| *child_pid == pid)
    {
        return Err((
            DirectoryWorkflowError::InstanceNotDirectChild(pid).into(),
            MetricReason::InvalidChild,
        ));
    }

    let Some(record) = SubnetRegistryOps::get(pid) else {
        return Err((
            DirectoryWorkflowError::RegistryEntryMissing(pid).into(),
            MetricReason::RegistryMissing,
        ));
    };

    if record.role != *expected_role {
        return Err((
            DirectoryWorkflowError::InstanceRoleMismatch {
                pid,
                expected: expected_role.clone(),
                actual: record.role,
            }
            .into(),
            MetricReason::RoleMismatch,
        ));
    }

    Ok(())
}
