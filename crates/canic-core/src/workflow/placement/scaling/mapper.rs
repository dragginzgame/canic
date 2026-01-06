use crate::{
    dto::placement::scaling::WorkerEntryView, ops::storage::placement::scaling::WorkerEntry,
};

///
/// ScalingMapper
///

pub struct ScalingMapper;

impl ScalingMapper {
    #[must_use]
    pub fn worker_entry_to_view(entry: &WorkerEntry) -> WorkerEntryView {
        WorkerEntryView {
            pool: entry.pool.to_string(),
            canister_role: entry.canister_role.clone(),
            created_at_secs: entry.created_at_secs,
        }
    }
}
