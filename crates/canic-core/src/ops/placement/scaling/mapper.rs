use crate::{
    dto::placement::scaling::WorkerEntry, storage::stable::scaling::WorkerEntryRecord,
    view::placement::scaling::ScalingWorkerPlanEntry,
};

///
/// WorkerEntryRecordMapper
///

pub struct WorkerEntryRecordMapper;

impl WorkerEntryRecordMapper {
    #[must_use]
    pub fn validated_to_record(
        plan: ScalingWorkerPlanEntry,
        created_at_secs: u64,
    ) -> WorkerEntryRecord {
        WorkerEntryRecord {
            pool: plan.pool,
            canister_role: plan.canister_role,
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
