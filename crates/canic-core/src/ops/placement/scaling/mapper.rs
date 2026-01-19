use crate::{
    domain::policy::placement::scaling::ScalingWorkerPlanEntry,
    dto::placement::scaling::WorkerEntry, ops::ic::IcOps,
    storage::stable::scaling::WorkerEntryRecord,
};

///
/// WorkerEntryRecordMapper
///

pub struct WorkerEntryRecordMapper;

impl WorkerEntryRecordMapper {
    #[must_use]
    pub fn validated_to_record(plan: ScalingWorkerPlanEntry) -> WorkerEntryRecord {
        WorkerEntryRecord {
            pool: plan.pool,
            canister_role: plan.canister_role,
            created_at_secs: IcOps::now_secs(),
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
