//! Module: ops::placement::scaling::mapper
//!
//! Responsibility: convert scaling worker records and plans into boundary views.
//! Does not own: scaling policy, worker registry mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by scaling workflows and storage facades.

use crate::{
    dto::placement::scaling::WorkerEntry, model::placement::scaling::ScalingWorkerEntry,
    storage::stable::scaling::WorkerEntryRecord,
};

///
/// WorkerEntryRecordMapper
///
/// Operations-layer mapper for scaling worker entries.
///

pub struct WorkerEntryRecordMapper;

impl WorkerEntryRecordMapper {
    #[must_use]
    pub fn validated_to_record(
        entry: ScalingWorkerEntry,
        created_at_secs: u64,
    ) -> WorkerEntryRecord {
        WorkerEntryRecord {
            pool: entry.pool,
            canister_role: entry.canister_role,
            created_at_secs,
        }
    }

    #[must_use]
    pub fn record_to_view(entry: &WorkerEntryRecord) -> WorkerEntry {
        WorkerEntry {
            pool: entry.pool.to_string(),
            canister_role: entry.canister_role.clone(),
            created_at_secs: entry.created_at_secs,
        }
    }
}
