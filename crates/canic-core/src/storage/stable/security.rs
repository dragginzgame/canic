use crate::{
    cdk::structures::{
        DefaultMemoryImpl,
        log::{Log as StableLogImpl, WriteError},
        memory::VirtualMemory,
    },
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{
        StorageError,
        prelude::*,
        stable::{
            StableMemoryError,
            memory::security::{SECURITY_EVENT_DATA_ID, SECURITY_EVENT_INDEX_ID},
        },
    },
};
use std::{cell::RefCell, collections::VecDeque};

///
/// StableSecurityEventLog
///

type StableSecurityEventLog = StableLogImpl<
    SecurityEventRecord,
    VirtualMemory<DefaultMemoryImpl>,
    VirtualMemory<DefaultMemoryImpl>,
>;

///
/// SecurityEventIndexMemory
///

struct SecurityEventIndexMemory;

///
/// SecurityEventDataMemory
///

struct SecurityEventDataMemory;

///
/// SecurityEventMemory
///

#[derive(Clone)]
struct SecurityEventMemory {
    index: VirtualMemory<DefaultMemoryImpl>,
    data: VirtualMemory<DefaultMemoryImpl>,
}

impl SecurityEventMemory {
    fn new() -> Self {
        Self {
            index: ic_memory!(SecurityEventIndexMemory, SECURITY_EVENT_INDEX_ID),
            data: ic_memory!(SecurityEventDataMemory, SECURITY_EVENT_DATA_ID),
        }
    }
}

eager_static! {
    static SECURITY_EVENT_MEMORY: SecurityEventMemory = SecurityEventMemory::new();
}

fn create_log() -> StableSecurityEventLog {
    SECURITY_EVENT_MEMORY.with(|mem| StableLogImpl::new(mem.index.clone(), mem.data.clone()))
}

eager_static! {
    static SECURITY_EVENTS: RefCell<StableSecurityEventLog> = RefCell::new(create_log());
}

fn with_events<R>(f: impl FnOnce(&StableSecurityEventLog) -> R) -> R {
    SECURITY_EVENTS.with_borrow(f)
}

fn with_events_mut<R>(f: impl FnOnce(&mut StableSecurityEventLog) -> R) -> R {
    SECURITY_EVENTS.with_borrow_mut(f)
}

///
/// SecurityEventRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecurityEventRecord {
    pub id: u64,
    pub created_at: u64,
    pub caller: Principal,
    pub endpoint: String,
    pub request_bytes: u64,
    pub max_bytes: u64,
    pub reason: SecurityEventReasonRecord,
}

impl_storable_unbounded!(SecurityEventRecord);

///
/// SecurityEventReasonRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SecurityEventReasonRecord {
    IngressPayloadLimitExceeded,
}

///
/// SecurityEventStore
///

pub struct SecurityEventStore;

impl SecurityEventStore {
    /// Append one security event and keep only the newest retained rows.
    pub(crate) fn append(
        max_entries: usize,
        entry: SecurityEventRecord,
    ) -> Result<u64, StorageError> {
        if max_entries == 0 {
            return Ok(0);
        }

        let id = append_raw(&entry)?;
        apply_retention(max_entries)?;

        Ok(id)
    }

    /// Return a point-in-time snapshot of stored security events.
    #[must_use]
    pub(crate) fn snapshot() -> Vec<SecurityEventRecord> {
        let mut out = Vec::new();
        with_events(|events| {
            for entry in events.iter() {
                out.push(entry);
            }
        });

        out
    }
}

// Append one raw stable log row and map memory failures to storage errors.
fn append_raw(entry: &SecurityEventRecord) -> Result<u64, StorageError> {
    with_events(|events| events.append(entry)).map_err(|e| StorageError::from(map_write_error(e)))
}

// Retain the newest entries and rebuild the stable log when the window overflows.
fn apply_retention(max_entries: usize) -> Result<(), StorageError> {
    let len = with_events(StableSecurityEventLog::len);
    if len <= u64::try_from(max_entries).unwrap_or(u64::MAX) {
        return Ok(());
    }

    let mut retained = VecDeque::new();
    with_events(|events| {
        for entry in events.iter() {
            retained.push_back(entry);
            if retained.len() > max_entries {
                retained.pop_front();
            }
        }
    });

    with_events_mut(|events| *events = create_log());
    for entry in retained {
        append_raw(&entry)?;
    }

    Ok(())
}

const fn map_write_error(err: WriteError) -> StableMemoryError {
    match err {
        WriteError::GrowFailed {
            current_size,
            delta,
        } => StableMemoryError::SecurityEventWriteFailed {
            current_size,
            delta,
        },
    }
}
