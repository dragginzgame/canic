use crate::{
    InternalError,
    ops::prelude::*,
    storage::stable::pool::{
        PoolData, PoolRecord, PoolRecordState, PoolStatus as ModelPoolStatus, PoolStore,
    },
};

///
/// PoolSnapshot
/// Internal, operational snapshot of the pool.
///

pub struct PoolSnapshot {
    pub entries: Vec<PoolEntrySnapshot>,
}

impl From<PoolData> for PoolSnapshot {
    fn from(data: PoolData) -> Self {
        Self {
            entries: data.entries.into_iter().map(Into::into).collect(),
        }
    }
}

///
/// PoolEntrySnapshot
/// Identity-carrying snapshot of a single pool entry.
///

pub struct PoolEntrySnapshot {
    pub pid: Principal,
    pub created_at: u64,
    pub cycles: Cycles,
    pub status: PoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolStatus {
    PendingReset,
    Ready,
    Failed { reason: String },
}

impl From<(Principal, PoolRecord)> for PoolEntrySnapshot {
    fn from((pid, record): (Principal, PoolRecord)) -> Self {
        Self {
            pid,
            created_at: record.header.created_at,
            cycles: record.state.cycles,
            status: record.state.status.into(),
            role: record.state.role,
            parent: record.state.parent,
            module_hash: record.state.module_hash,
        }
    }
}

///
/// PoolOps
/// Operational façade over pool storage.
///

pub struct PoolOps;

impl PoolOps {
    // ---------------------------------------------------------------
    // Registration
    // ---------------------------------------------------------------

    pub fn register_ready(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
        created_at: u64,
    ) {
        PoolStore::register(
            pid,
            cycles,
            ModelPoolStatus::Ready,
            role,
            parent,
            module_hash,
            created_at,
        );
    }

    // ---------------------------------------------------------------
    // State transitions
    // ---------------------------------------------------------------

    pub fn mark_pending_reset(pid: Principal, created_at: u64) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            PoolStatus::PendingReset,
            None,
            created_at,
        );
    }

    pub fn mark_ready(pid: Principal, cycles: Cycles, created_at: u64) {
        Self::register_or_update_state(pid, cycles, PoolStatus::Ready, None, created_at);
    }

    pub fn mark_failed(pid: Principal, err: &InternalError, created_at: u64) {
        Self::register_or_update_state(
            pid,
            Cycles::default(),
            PoolStatus::Failed {
                reason: err.to_string(),
            },
            None,
            created_at,
        );
    }

    // ---------------------------------------------------------------
    // Snapshot (read-only)
    // ---------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> PoolSnapshot {
        PoolStore::export().into()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        PoolStore::contains(pid)
    }

    pub fn has_pending_reset() -> bool {
        PoolStore::has_status(ModelPoolStatus::PendingReset)
    }

    // ---------------------------------------------------------------
    // Removal
    // ---------------------------------------------------------------

    pub fn remove(pid: &Principal) {
        PoolStore::remove(pid);
    }

    // ---------------------------------------------------------------
    // Internal helper
    // ---------------------------------------------------------------

    fn register_or_update_state(
        pid: Principal,
        cycles: Cycles,
        status: PoolStatus,
        role: Option<CanisterRole>,
        created_at: u64,
    ) {
        let model_status = status_to_model(&status);
        let updated = PoolStore::update_state_with(pid, |mut state: PoolRecordState| {
            state.cycles = cycles.clone();
            state.status = model_status.clone();

            if role.is_some() {
                state.role.clone_from(&role);
            }

            state
        });

        if !updated {
            PoolStore::register(pid, cycles, model_status, role, None, None, created_at);
        }
    }
}

fn status_to_model(status: &PoolStatus) -> ModelPoolStatus {
    match status {
        PoolStatus::PendingReset => ModelPoolStatus::PendingReset,
        PoolStatus::Ready => ModelPoolStatus::Ready,
        PoolStatus::Failed { reason } => ModelPoolStatus::Failed {
            reason: reason.clone(),
        },
    }
}

impl From<ModelPoolStatus> for PoolStatus {
    fn from(status: ModelPoolStatus) -> Self {
        match status {
            ModelPoolStatus::PendingReset => Self::PendingReset,
            ModelPoolStatus::Ready => Self::Ready,
            ModelPoolStatus::Failed { reason } => Self::Failed { reason },
        }
    }
}
