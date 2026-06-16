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
    domain::policy::pool::authority::require_pool_admin,
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
    workflow::prelude::*,
};

///
/// PoolWorkflow
///

pub struct PoolWorkflow;

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
