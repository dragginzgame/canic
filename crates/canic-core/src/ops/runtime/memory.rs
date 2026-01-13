//! Runtime memory registry primitives.
//! Owns TLS setup for memory registry initialization.

use crate::{
    CRATE_NAME, InternalError,
    ops::runtime::RuntimeOpsError,
    storage::stable::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
};
use canic_memory::{
    registry::MemoryRegistryError,
    runtime::{
        init_eager_tls,
        registry::{MemoryRegistryInitSummary as RawInitSummary, MemoryRegistryRuntime},
    },
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    // this error comes from the canic-memory crate
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
}

impl From<MemoryRegistryOpsError> for InternalError {
    fn from(err: MemoryRegistryOpsError) -> Self {
        RuntimeOpsError::MemoryRegistryOps(err).into()
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
/// MemoryRegistryOps
///

pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
    pub fn init_eager_tls() {
        init_eager_tls();
    }

    pub(crate) fn init_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
        let summary =
            MemoryRegistryRuntime::init(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryRegistryOpsError::from)?;

        Ok(MemoryRegistryInitSummary::from_raw(summary))
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
}
