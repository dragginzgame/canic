//! Module: workflow::pool::recycle
//!
//! Responsibility: recycle registered canisters into the reset pool.
//! Does not own: endpoint authorization, stable pool schemas, or pool policy rules.
//! Boundary: workflow helper coordinating topology removal, reset, storage, scheduling, and metrics.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::pool::CanisterPoolStatus,
    ops::{
        ic::IcOps,
        runtime::metrics::{
            pool::{PoolMetricOperation as MetricOperation, PoolMetricReason as MetricReason},
            recording::PoolMetricEvent as MetricEvent,
        },
        storage::{
            pool::{PoolOps, PoolRegistrationMetadata},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::pool::{PoolWorkflow, query::PoolQuery, scheduler::PoolSchedulerWorkflow},
};

impl PoolWorkflow {
    pub async fn pool_recycle_canister(pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::Recycle);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::Recycle, &err);
            return Err(err);
        }
        if pool_recycle_already_present(pid) {
            MetricEvent::skipped(MetricOperation::Recycle, MetricReason::AlreadyPresent);
            return Ok(());
        }

        // Recycling a missing child is an idempotent no-op so stale directory cleanup
        // never depends on the provisional child still existing.
        let Some(metadata) = PoolRegistrationMetadata::from_subnet_registry(pid) else {
            MetricEvent::skipped(MetricOperation::Recycle, MetricReason::NotFound);
            return Ok(());
        };

        // Remove from topology and record the pending pool entry before the
        // destructive reset, so duplicate retries cannot re-enter the reset path.
        if !SubnetRegistryOps::unregister(&pid) {
            let err = InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("pool recycle topology removal missing for {pid}"),
            );
            MetricEvent::failed(MetricOperation::Recycle, &err);
            return Err(err);
        }
        mark_pool_recycle_pending(pid, &metadata, IcOps::now_secs());

        // Destructive reset
        let cycles = match Self::reset_into_pool(pid).await {
            Ok(cycles) => cycles,
            Err(err) => {
                PoolSchedulerWorkflow::schedule();
                MetricEvent::failed(MetricOperation::Recycle, &err);
                return Err(err);
            }
        };

        // Register back into pool, preserving metadata
        let created_at = IcOps::now_secs();
        PoolOps::register_ready_with_metadata(pid, cycles, &metadata, created_at);

        MetricEvent::completed(MetricOperation::Recycle, MetricReason::Ok);

        Ok(())
    }
}

fn pool_recycle_already_present(pid: Principal) -> bool {
    matches!(
        PoolQuery::pool_entry(pid).map(|entry| entry.status),
        Some(CanisterPoolStatus::PendingReset | CanisterPoolStatus::Ready)
    )
}

fn mark_pool_recycle_pending(pid: Principal, metadata: &PoolRegistrationMetadata, created_at: u64) {
    PoolOps::register_pending_reset_with_metadata(pid, metadata, created_at);
}

#[cfg(test)]
mod tests {
    use crate::cdk::types::Cycles;

    use super::*;
    use crate::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn pool_recycle_detects_pending_reset_before_reset() {
        let pid = p(50);
        PoolOps::remove(&pid);

        assert!(!pool_recycle_already_present(pid));

        PoolOps::mark_pending_reset(pid, 100);

        assert!(pool_recycle_already_present(pid));
        assert_eq!(
            PoolQuery::pool_list()
                .entries
                .iter()
                .filter(|entry| entry.pid == pid)
                .count(),
            1,
            "duplicate recycle must not create another pending entry"
        );
        assert_eq!(
            PoolQuery::pool_entry(pid).expect("pending entry").status,
            CanisterPoolStatus::PendingReset
        );

        PoolOps::remove(&pid);
    }

    #[test]
    fn pool_recycle_detects_ready_canister_before_reset() {
        let pid = p(53);
        PoolOps::remove(&pid);

        assert!(!pool_recycle_already_present(pid));

        PoolOps::register_ready(pid, Cycles::new(10), None, None, None, 100);

        assert!(pool_recycle_already_present(pid));
        assert_eq!(
            PoolQuery::pool_entry(pid).expect("ready entry").status,
            CanisterPoolStatus::Ready
        );

        PoolOps::remove(&pid);
    }

    #[test]
    fn pool_recycle_pending_entry_preserves_registry_metadata() {
        let root = p(51);
        let pid = p(52);
        let role = CanisterRole::new("recyclable");
        let module_hash = vec![1, 2, 3, 4];

        PoolOps::remove(&pid);
        let _ = SubnetRegistryOps::unregister(&pid);
        let _ = SubnetRegistryOps::unregister(&root);
        SubnetRegistryOps::register_root(root, 100);
        SubnetRegistryOps::register_unchecked(pid, &role, root, module_hash.clone(), 101)
            .expect("child registered");

        let metadata =
            PoolRegistrationMetadata::from_subnet_registry(pid).expect("registry metadata");
        let _ = SubnetRegistryOps::unregister(&pid);
        mark_pool_recycle_pending(pid, &metadata, 102);

        assert!(pool_recycle_already_present(pid));
        assert!(!SubnetRegistryOps::is_registered(pid));

        let pool_entry = PoolQuery::pool_entry(pid).expect("pending pool entry");
        assert_eq!(pool_entry.status, CanisterPoolStatus::PendingReset);
        assert_eq!(pool_entry.role, Some(role));
        assert_eq!(pool_entry.parent, Some(root));
        assert_eq!(pool_entry.module_hash, Some(module_hash));
        assert_eq!(
            PoolQuery::pool_list()
                .entries
                .iter()
                .filter(|entry| entry.pid == pid)
                .count(),
            1,
            "recycle preparation must keep one pool entry"
        );

        PoolOps::remove(&pid);
        let _ = SubnetRegistryOps::unregister(&pid);
        let _ = SubnetRegistryOps::unregister(&root);
    }

    #[test]
    fn pool_recovery_selection_retains_pending_metadata_until_transition() {
        let root = p(54);
        let pid = p(55);
        let role = CanisterRole::new("recovery_metadata");
        let module_hash = vec![5, 4, 3, 2];

        PoolOps::remove(&pid);
        let _ = SubnetRegistryOps::unregister(&pid);
        let _ = SubnetRegistryOps::unregister(&root);
        SubnetRegistryOps::register_root(root, 200);
        SubnetRegistryOps::register_unchecked(pid, &role, root, module_hash.clone(), 201)
            .expect("child registered");

        let metadata =
            PoolRegistrationMetadata::from_subnet_registry(pid).expect("registry metadata");
        assert!(SubnetRegistryOps::unregister(&pid));
        mark_pool_recycle_pending(pid, &metadata, 202);

        let selected = PoolOps::oldest_pending_reset_pids(usize::MAX);
        assert!(selected.contains(&pid));

        let pending = PoolQuery::pool_entry(pid).expect("pending pool entry retained");
        assert_eq!(pending.status, CanisterPoolStatus::PendingReset);
        assert_eq!(pending.role, Some(role.clone()));
        assert_eq!(pending.parent, Some(root));
        assert_eq!(pending.module_hash, Some(module_hash.clone()));

        PoolWorkflow::mark_ready(pid, Cycles::new(500));

        let ready = PoolQuery::pool_entry(pid).expect("ready pool entry");
        assert_eq!(ready.status, CanisterPoolStatus::Ready);
        assert_eq!(ready.cycles, Cycles::new(500));
        assert_eq!(ready.role, Some(role.clone()));
        assert_eq!(ready.parent, Some(root));
        assert_eq!(ready.module_hash, Some(module_hash.clone()));

        let failure = InternalError::workflow(InternalErrorOrigin::Workflow, "reset failed");
        PoolWorkflow::mark_failed(pid, &failure);

        let failed = PoolQuery::pool_entry(pid).expect("failed pool entry");
        assert!(matches!(failed.status, CanisterPoolStatus::Failed { .. }));
        assert_eq!(failed.role, Some(role));
        assert_eq!(failed.parent, Some(root));
        assert_eq!(failed.module_hash, Some(module_hash));

        PoolOps::remove(&pid);
        let _ = SubnetRegistryOps::unregister(&pid);
        let _ = SubnetRegistryOps::unregister(&root);
    }
}
