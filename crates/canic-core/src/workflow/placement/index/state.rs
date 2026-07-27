//! Module: workflow::placement::index::state
//!
//! Responsibility: hold index workflow errors, classifications, and validators.
//! Does not own: index registry mutation, child creation, or endpoint DTOs.
//! Boundary: provides workflow-local state helpers for index orchestration.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::IndexConfig,
    ids::CanisterRole,
    ops::{
        ic::IcOps,
        runtime::metrics::placement_index::PlacementIndexMetricReason as MetricReason,
        storage::{
            children::CanisterChildrenOps, placement::index::PlacementIndexRegistryOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error as ThisError;

///
/// PlacementIndexWorkflowError
///
/// Workflow-local failures raised while coordinating index placement.
///

#[derive(Debug, ThisError)]
pub(super) enum PlacementIndexWorkflowError {
    #[error("index placement is not configured for the current canister")]
    IndexDisabled,

    #[error("unknown index pool '{requested}': configured pools: {available}")]
    UnknownPool {
        requested: String,
        available: String,
    },

    #[error("instance {0} is not a direct child of the current canister")]
    InstanceNotDirectChild(Principal),

    #[error("index instance {pid} has role '{actual}', expected '{expected}'")]
    InstanceRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        actual: CanisterRole,
    },

    #[error("index instance {0} is not present in the subnet registry")]
    RegistryEntryMissing(Principal),
}

impl From<PlacementIndexWorkflowError> for InternalError {
    fn from(err: PlacementIndexWorkflowError) -> Self {
        Self::domain(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// PlacementIndexEntryClassification
///
/// Snapshot classification used to choose the next index workflow step.
///

#[derive(Debug, Eq, PartialEq)]
pub(super) enum PlacementIndexEntryClassification {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingFresh {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Repairable {
        claim_id: u64,
        owner_pid: Principal,
        provisional_pid: Principal,
    },
    Resumable {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
    },
    NeedsCleanup {
        claim_id: u64,
        owner_pid: Principal,
        provisional_pid: Principal,
    },
}

static PLACEMENT_INDEX_CLAIM_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn available_pool_names(index: &IndexConfig) -> String {
    if index.pools.is_empty() {
        return "none".to_string();
    }

    let mut names: Vec<_> = index.pools.keys().cloned().collect();
    names.sort();
    names.join(", ")
}

pub(super) fn new_claim_id() -> u64 {
    let nonce = PLACEMENT_INDEX_CLAIM_NONCE.fetch_add(1, Ordering::Relaxed);
    IcOps::now_millis().rotate_left(21) ^ nonce
}

pub(super) const fn pending_is_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > PlacementIndexRegistryOps::PENDING_TTL_SECS
}

// Validate a bind target while preserving a bounded metric reason for callers.
pub(super) fn validate_bind_target_with_reason(
    pid: Principal,
    expected_role: &CanisterRole,
) -> Result<(), (InternalError, MetricReason)> {
    if !CanisterChildrenOps::data()
        .entries
        .iter()
        .any(|entry| entry.pid == pid)
    {
        return Err((
            PlacementIndexWorkflowError::InstanceNotDirectChild(pid).into(),
            MetricReason::InvalidChild,
        ));
    }

    let Some((actual_role, _)) = SubnetRegistryOps::role_parent(pid) else {
        return Err((
            PlacementIndexWorkflowError::RegistryEntryMissing(pid).into(),
            MetricReason::RegistryMissing,
        ));
    };

    if actual_role != *expected_role {
        return Err((
            PlacementIndexWorkflowError::InstanceRoleMismatch {
                pid,
                expected: expected_role.clone(),
                actual: actual_role,
            }
            .into(),
            MetricReason::RoleMismatch,
        ));
    }

    Ok(())
}
