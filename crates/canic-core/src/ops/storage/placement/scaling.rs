//! Module: ops::storage::placement::scaling
//!
//! Responsibility: provide deterministic access to scaling worker registry records.
//! Does not own: scaling policy, worker orchestration, or endpoint DTOs.
//! Boundary: storage ops facade over stable scaling registry records.

use crate::{
    dto::placement::scaling::{ScalingRegistryEntry, ScalingRegistryResponse},
    model::placement::scaling::ScalingWorkerEntry,
    ops::{placement::scaling::mapper::WorkerEntryRecordMapper, prelude::*},
    storage::stable::scaling::ScalingRegistry,
};

///
/// ScalingRegistryOps
///
/// Storage-ops facade for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    fn entry_is_current(
        record: &crate::storage::stable::scaling::ScalingRegistryEntryRecord,
    ) -> bool {
        crate::ops::storage::children::CanisterChildrenOps::matches_allocation(
            record.pid,
            record.entry.allocation_operation_id,
        )
    }

    pub fn upsert(
        pid: Principal,
        worker: ScalingWorkerEntry,
        created_at_secs: u64,
        allocation_operation_id: [u8; 32],
    ) {
        let entry = WorkerEntryRecordMapper::validated_to_record(
            worker,
            created_at_secs,
            allocation_operation_id,
        );
        ScalingRegistry::upsert(pid, entry);
    }

    #[must_use]
    pub fn count_by_pool(pool: &str) -> u32 {
        u32::try_from(
            ScalingRegistry::export()
                .entries
                .into_iter()
                .filter(|record| {
                    record.entry.pool.as_ref() == pool && Self::entry_is_current(record)
                })
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    #[must_use]
    pub fn entries_response() -> ScalingRegistryResponse {
        let entries = ScalingRegistry::export()
            .entries
            .into_iter()
            .filter(Self::entry_is_current)
            .map(|record| ScalingRegistryEntry {
                pid: record.pid,
                entry: WorkerEntryRecordMapper::record_to_view(&record.entry),
            })
            .collect();

        ScalingRegistryResponse(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::BoundedString64,
        ops::storage::children::CanisterChildrenOps,
        test::{
            seams::{lock, p},
            support::direct_child,
        },
    };

    #[test]
    fn recycled_worker_is_not_counted_or_returned_until_registered_for_its_allocation() {
        let _guard = lock();
        let pid = p(17);
        let role = CanisterRole::new("worker");
        let worker = ScalingWorkerEntry {
            pool: BoundedString64::new("reuse"),
            canister_role: role.clone(),
        };
        CanisterChildrenOps::import_direct_children(
            p(2),
            vec![direct_child(pid, role.clone(), [1; 32])],
        );
        ScalingRegistryOps::upsert(pid, worker.clone(), 1, [1; 32]);
        assert_eq!(ScalingRegistryOps::count_by_pool("reuse"), 1);
        CanisterChildrenOps::import_direct_children(p(2), vec![direct_child(pid, role, [2; 32])]);
        assert_eq!(ScalingRegistryOps::count_by_pool("reuse"), 0);
        assert!(
            !ScalingRegistryOps::entries_response()
                .0
                .iter()
                .any(|record| record.pid == pid)
        );
        ScalingRegistryOps::upsert(pid, worker, 2, [2; 32]);
        assert_eq!(ScalingRegistryOps::count_by_pool("reuse"), 1);
        assert!(
            ScalingRegistryOps::entries_response()
                .0
                .iter()
                .any(|record| record.pid == pid)
        );
    }
}
