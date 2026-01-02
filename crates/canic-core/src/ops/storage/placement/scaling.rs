use crate::{
    cdk::types::{BoundedString64, Principal},
    ids::CanisterRole,
    storage::memory::scaling::{
        ScalingRegistry, ScalingRegistryData as ModelScalingRegistryData,
        WorkerEntry as ModelWorkerEntry,
    },
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerEntry {
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
    pub created_at_secs: u64,
}

#[derive(Clone, Debug)]
pub struct ScalingRegistrySnapshot {
    pub entries: Vec<(Principal, WorkerEntry)>,
}

impl ScalingRegistryOps {
    pub(crate) fn upsert(pid: Principal, entry: WorkerEntry) {
        ScalingRegistry::upsert(pid, entry.into());
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub(crate) fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        ScalingRegistry::export()
            .entries
            .into_iter()
            .filter(|(_, entry)| entry.pool.as_ref() == pool)
            .map(|(pid, entry)| (pid, entry.into()))
            .collect()
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn count_by_pool(pool: &str) -> u32 {
        Self::find_by_pool(pool).len() as u32
    }

    #[must_use]
    pub fn export() -> ScalingRegistrySnapshot {
        ScalingRegistry::export().into()
    }
}

impl From<ModelScalingRegistryData> for ScalingRegistrySnapshot {
    fn from(data: ModelScalingRegistryData) -> Self {
        Self {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, entry)| (pid, entry.into()))
                .collect(),
        }
    }
}

impl From<ModelWorkerEntry> for WorkerEntry {
    fn from(entry: ModelWorkerEntry) -> Self {
        Self {
            pool: entry.pool,
            canister_role: entry.canister_role,
            created_at_secs: entry.created_at_secs,
        }
    }
}

impl From<WorkerEntry> for ModelWorkerEntry {
    fn from(entry: WorkerEntry) -> Self {
        Self {
            pool: entry.pool,
            canister_role: entry.canister_role,
            created_at_secs: entry.created_at_secs,
        }
    }
}
