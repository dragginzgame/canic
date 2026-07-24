//! Module: workflow::placement::binding::state
//!
//! Responsibility: hold binding workflow errors, classifications, and validators.
//! Does not own: binding registry mutation, child creation, or endpoint DTOs.
//! Boundary: provides workflow-local state helpers for binding orchestration.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::BindingConfig,
    ids::CanisterRole,
    ops::{
        ic::IcOps,
        runtime::metrics::placement_binding::PlacementBindingMetricReason as MetricReason,
        storage::{
            children::CanisterChildrenOps, placement::binding::PlacementBindingRegistryOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error as ThisError;

///
/// PlacementBindingWorkflowError
///
/// Workflow-local failures raised while coordinating binding placement.
///

#[derive(Debug, ThisError)]
pub(super) enum PlacementBindingWorkflowError {
    #[error("binding placement is not configured for the current canister")]
    BindingDisabled,

    #[error("unknown binding pool '{requested}': configured pools: {available}")]
    UnknownPool {
        requested: String,
        available: String,
    },

    #[error("instance {0} is not a direct child of the current canister")]
    InstanceNotDirectChild(Principal),

    #[error("binding instance {pid} has role '{actual}', expected '{expected}'")]
    InstanceRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        actual: CanisterRole,
    },

    #[error("binding instance {0} is not present in the subnet registry")]
    RegistryEntryMissing(Principal),
}

impl From<PlacementBindingWorkflowError> for InternalError {
    fn from(err: PlacementBindingWorkflowError) -> Self {
        Self::domain(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// PlacementBindingEntryClassification
///
/// Snapshot classification used to choose the next binding workflow step.
///

#[derive(Debug, Eq, PartialEq)]
pub(super) enum PlacementBindingEntryClassification {
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

static PLACEMENT_BINDING_CLAIM_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn available_pool_names(binding: &BindingConfig) -> String {
    if binding.pools.is_empty() {
        return "none".to_string();
    }

    let mut names: Vec<_> = binding.pools.keys().cloned().collect();
    names.sort();
    names.join(", ")
}

pub(super) fn new_claim_id() -> u64 {
    let nonce = PLACEMENT_BINDING_CLAIM_NONCE.fetch_add(1, Ordering::Relaxed);
    IcOps::now_millis().rotate_left(21) ^ nonce
}

pub(super) const fn pending_is_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > PlacementBindingRegistryOps::PENDING_TTL_SECS
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
            PlacementBindingWorkflowError::InstanceNotDirectChild(pid).into(),
            MetricReason::InvalidChild,
        ));
    }

    let Some((actual_role, _)) = SubnetRegistryOps::role_parent(pid) else {
        return Err((
            PlacementBindingWorkflowError::RegistryEntryMissing(pid).into(),
            MetricReason::RegistryMissing,
        ));
    };

    if actual_role != *expected_role {
        return Err((
            PlacementBindingWorkflowError::InstanceRoleMismatch {
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
