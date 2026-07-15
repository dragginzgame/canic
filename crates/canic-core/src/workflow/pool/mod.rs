//! Module: workflow::pool
//!
//! Responsibility: coordinate pool canister selection, reset, import, and admin flows.
//! Does not own: endpoint authorization, stable pool records, or pool policy decisions.
//! Boundary: workflow layer coordinating pool storage, policy, IC ops, and metrics.

pub mod admin;
pub mod admissibility;
pub mod controllers;
mod create_empty;
mod import;
pub mod query;
mod recycle;
mod reset;
pub mod scheduler;

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
    domain::policy::pure::pool::{PoolPolicyError, authority::require_pool_admin},
    ops::{
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::{
                pool::{PoolMetricOperation as MetricOperation, PoolMetricReason as MetricReason},
                recording::PoolMetricEvent as MetricEvent,
            },
        },
        storage::pool::PoolOps,
    },
};

///
/// PoolWorkflow
///

pub struct PoolWorkflow;

#[must_use]
const fn metric_reason_for_policy(err: &PoolPolicyError) -> MetricReason {
    match err {
        PoolPolicyError::RegisteredInSubnet(_) => MetricReason::RegisteredInSubnet,
        PoolPolicyError::NonImportableOnLocal { .. } => MetricReason::NonImportableLocal,
        PoolPolicyError::NotRegisteredInSubnet(_) => MetricReason::NotFound,
        PoolPolicyError::NotAuthorized => MetricReason::PolicyDenied,
    }
}

impl PoolWorkflow {
    fn mark_pending_reset(pid: Principal) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_pending_reset(pid, created_at);
    }

    fn mark_ready(pid: Principal, cycles: Cycles) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_ready(pid, cycles, created_at);
    }

    fn mark_failed(pid: Principal, err: &InternalError) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_failed(pid, err, created_at);
    }

    #[must_use]
    pub fn pop_oldest_ready() -> Option<Principal> {
        let pid = PoolOps::pop_oldest_ready_pid();
        if pid.is_some() {
            MetricEvent::completed(MetricOperation::SelectReady, MetricReason::Ok);
        } else {
            MetricEvent::skipped(MetricOperation::SelectReady, MetricReason::Empty);
        }
        pid
    }

    #[must_use]
    pub fn pop_oldest_pending_reset() -> Option<Principal> {
        PoolOps::pop_oldest_pending_reset_pid()
    }

    fn require_pool_admin() -> Result<(), InternalError> {
        require_pool_admin(EnvOps::is_root()).map_err(Into::into)
    }
}
