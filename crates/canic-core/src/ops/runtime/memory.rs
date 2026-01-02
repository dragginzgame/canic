//! Runtime memory registry primitives.
//! Exposes init/snapshot helpers without logging or DTO construction.

use crate::{
    CRATE_NAME, Error, ThisError,
    ops::runtime::RuntimeOpsError,
    storage::memory::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
};
use canic_memory::{
    registry::MemoryRegistryError,
    runtime::registry::{MemoryRegistryInitSummary as RawInitSummary, MemoryRegistryRuntime},
};

///
/// MemoryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryOpsError {
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
}

impl From<MemoryOpsError> for Error {
    fn from(err: MemoryOpsError) -> Self {
        RuntimeOpsError::MemoryOps(err).into()
    }
}

///
/// MemoryRangeSnapshot
///

#[derive(Clone, Debug)]
pub struct MemoryRangeSnapshot {
    pub crate_name: String,
    pub start: u8,
    pub end: u8,
}

///
/// MemoryRegistryEntrySnapshot
///

#[derive(Clone, Debug)]
pub struct MemoryRegistryEntrySnapshot {
    pub id: u8,
    pub crate_name: String,
    pub label: String,
}

///
/// MemoryRegistryInitSummary
///

#[derive(Clone, Debug)]
pub struct MemoryRegistryInitSummary {
    pub ranges: Vec<MemoryRangeSnapshot>,
    pub entries: Vec<MemoryRegistryEntrySnapshot>,
}

impl MemoryRegistryInitSummary {
    fn from_raw(summary: RawInitSummary) -> Self {
        let ranges = summary
            .ranges
            .into_iter()
            .map(|(crate_name, range)| MemoryRangeSnapshot {
                crate_name,
                start: range.start,
                end: range.end,
            })
            .collect();

        let entries = summary
            .entries
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntrySnapshot {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
            })
            .collect();

        Self { ranges, entries }
    }
}

///
/// MemoryOps
///

pub struct MemoryOps;

impl MemoryOps {
    pub(crate) fn init_registry() -> Result<MemoryRegistryInitSummary, Error> {
        let summary =
            MemoryRegistryRuntime::init(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryOpsError::from)?;

        Ok(MemoryRegistryInitSummary::from_raw(summary))
    }

    #[must_use]
    pub fn snapshot_ranges() -> Vec<MemoryRangeSnapshot> {
        MemoryRegistryRuntime::snapshot_ranges()
            .into_iter()
            .map(|(crate_name, range)| MemoryRangeSnapshot {
                crate_name,
                start: range.start,
                end: range.end,
            })
            .collect()
    }

    #[must_use]
    pub fn snapshot_entries() -> Vec<MemoryRegistryEntrySnapshot> {
        MemoryRegistryRuntime::snapshot_entries()
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntrySnapshot {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
            })
            .collect()
    }

    #[must_use]
    pub fn get_entry(id: u8) -> Option<MemoryRegistryEntrySnapshot> {
        MemoryRegistryRuntime::get(id).map(|entry| MemoryRegistryEntrySnapshot {
            id,
            crate_name: entry.crate_name,
            label: entry.label,
        })
    }
}
