//! Runtime memory registry primitives.
//! Owns TLS setup for memory registry initialization.

use crate::{
    CRATE_NAME, InternalError,
    dto::memory::{
        MemoryLedgerResponse, MemoryRangeAuthorityEntry, MemoryRangeEntry, MemoryRegistryEntry,
    },
    memory::{
        api::MemoryApi,
        registry::MemoryRegistryError,
        runtime::{
            init_eager_tls,
            registry::{MemoryRegistryInitSummary as RawInitSummary, MemoryRegistryRuntime},
            run_registered_eager_init,
        },
    },
    ops::runtime::RuntimeOpsError,
    storage::stable::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    // this error comes from the Canic memory runtime boundary
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
/// MemoryRegistryInitSummary
///

#[derive(Clone, Debug)]
pub struct MemoryRegistryInitSummary {
    pub ranges: Vec<MemoryRangeSnapshot>,
    pub entries: Vec<MemoryRegistryEntry>,
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
            .map(|(id, entry)| MemoryRegistryEntry {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
                stable_key: entry.stable_key,
                schema_version: entry.schema_version,
                schema_fingerprint: entry.schema_fingerprint,
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
    // Run eager TLS touches after the registry validates stable-memory slots.
    pub fn init_eager_tls() {
        init_eager_tls();
    }

    // Run registered eager-init hooks before the registry commits deferred items.
    pub fn run_registered_eager_init() {
        run_registered_eager_init();
    }

    // Initialize the stable-memory registry for this crate and summarize the layout.
    pub(crate) fn init_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
        let summary =
            MemoryRegistryRuntime::init(Some((CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)))
                .map_err(MemoryRegistryOpsError::from)?;

        Ok(MemoryRegistryInitSummary::from_raw(summary))
    }

    // Run the full synchronous Canic memory bootstrap and return the committed layout.
    pub fn bootstrap_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
        Self::run_registered_eager_init();
        let summary = Self::init_registry()?;
        Self::init_eager_tls();
        Ok(summary)
    }

    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn is_initialized() -> bool {
        MemoryRegistryRuntime::is_initialized()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_bootstrap() -> Result<(), InternalError> {
        if Self::is_initialized() {
            return Ok(());
        }

        let _ = Self::bootstrap_registry()?;
        Ok(())
    }

    #[must_use]
    pub fn snapshot_entries() -> Vec<MemoryRegistryEntry> {
        MemoryRegistryRuntime::snapshot_entries()
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntry {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
                stable_key: entry.stable_key,
                schema_version: entry.schema_version,
                schema_fingerprint: entry.schema_fingerprint,
            })
            .collect()
    }

    // Read the committed ABI ledger using the restricted diagnostic path.
    pub fn ledger_snapshot() -> Result<MemoryLedgerResponse, InternalError> {
        let snapshot = MemoryApi::ledger_snapshot().map_err(MemoryRegistryOpsError::from)?;

        let authorities = snapshot
            .authorities
            .into_iter()
            .map(|authority| MemoryRangeAuthorityEntry {
                owner: authority.owner,
                start: authority.range.start,
                end: authority.range.end,
                purpose: authority.purpose,
            })
            .collect();

        let ranges = snapshot
            .ranges
            .into_iter()
            .map(|(owner, range)| MemoryRangeEntry {
                owner,
                start: range.start,
                end: range.end,
            })
            .collect();

        let entries = snapshot
            .entries
            .into_iter()
            .map(|(id, entry)| MemoryRegistryEntry {
                id,
                crate_name: entry.crate_name,
                label: entry.label,
                stable_key: entry.stable_key,
                schema_version: entry.schema_version,
                schema_fingerprint: entry.schema_fingerprint,
            })
            .collect();

        Ok(MemoryLedgerResponse {
            magic: snapshot.magic,
            format_id: snapshot.format_id,
            schema_version: snapshot.schema_version,
            layout_epoch: snapshot.layout_epoch,
            header_len: snapshot.header_len,
            header_checksum: snapshot.header_checksum,
            current_generation: snapshot.current_generation,
            authorities,
            ranges,
            entries,
        })
    }
}
