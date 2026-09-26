//! Retain finite initial inspection attempts under the existing Fleet operation lock.
//!
//! This owner never submits a request or replaces a successful observation with a newer baseline.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{read_optional_regular_bytes_bounded, write_bytes},
    fleet_ensure::{
        model::capacity_import::survey::{
            CapacityImportSampleRecord, CapacityImportSurveyCanisterRecord,
            CapacityImportSurveyRecord,
        },
        ops::{
            EnsurePaths,
            capacity_import::journal::{CapacityImportJournalError, CapacityImportJournalStore},
        },
    },
};
use candid::Principal;
use canic_core::{cdk::utils::hash::hex_bytes, ids::MAX_FLEET_CAPACITY_IMPORT_SOURCES};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

const MAXIMUM_BYTES: usize = 2 * 1024 * 1024;
/// Initial planning may issue at most this many status requests per exact physical ID.
pub const MAXIMUM_ATTEMPTS: u32 = 2;

/// Survey persistence borrows the exact Fleet lock for its complete lifetime.
pub struct CapacityImportSurveyStore<'a> {
    _owner: &'a CapacityImportJournalStore,
    path: PathBuf,
    record: CapacityImportSurveyRecord,
}

impl<'a> CapacityImportSurveyStore<'a> {
    /// Open or begin an exact-input survey without issuing an observation.
    pub fn open(
        owner: &'a CapacityImportJournalStore,
        paths: &EnsurePaths,
        request_sha256: [u8; 32],
        canisters: &[Principal],
    ) -> Result<Self, CapacityImportJournalError> {
        Self::open_scoped(
            owner,
            paths,
            request_sha256,
            canisters,
            MAX_FLEET_CAPACITY_IMPORT_SOURCES + 1,
            "capacity-import-surveys",
        )
    }

    /// Bootstrap observes infrastructure plus held pools, under the bounded Ensure inventory.
    pub(in crate::fleet_ensure) fn open_bootstrap(
        owner: &'a CapacityImportJournalStore,
        paths: &EnsurePaths,
        request_sha256: [u8; 32],
        canisters: &[Principal],
    ) -> Result<Self, CapacityImportJournalError> {
        Self::open_scoped(
            owner,
            paths,
            request_sha256,
            canisters,
            crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS,
            "infrastructure-bootstrap-surveys",
        )
    }

    fn open_scoped(
        owner: &'a CapacityImportJournalStore,
        paths: &EnsurePaths,
        request_sha256: [u8; 32],
        canisters: &[Principal],
        maximum: usize,
        directory: &str,
    ) -> Result<Self, CapacityImportJournalError> {
        let keys = canisters
            .iter()
            .map(Principal::to_text)
            .collect::<BTreeSet<_>>();
        if !owner.owns_paths(paths)
            || request_sha256 == [0; 32]
            || keys.is_empty()
            || keys.len() != canisters.len()
            || keys.len() > maximum
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        let path = paths
            .plan
            .with_file_name(directory)
            .join(format!("{}.json", hex_bytes(request_sha256)));
        let retained = read_optional_regular_bytes_bounded(&path, MAXIMUM_BYTES)
            .map_err(|_| CapacityImportJournalError::Integrity)?;
        let record = match retained {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => CapacityImportSurveyRecord {
                schema_version: 1,
                request_sha256,
                canisters: keys
                    .iter()
                    .map(|key| {
                        (
                            key.clone(),
                            CapacityImportSurveyCanisterRecord {
                                attempts: 0,
                                sample: None,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            },
        };
        validate(&record, request_sha256, &keys)?;
        let result = Self {
            _owner: owner,
            path,
            record,
        };
        result.save()?;
        Ok(result)
    }

    /// Return the original successful sample, including after an interrupted review build.
    #[must_use]
    pub fn sample(&self, canister: Principal) -> Option<&CapacityImportSampleRecord> {
        self.record
            .canisters
            .get(&canister.to_text())
            .and_then(|entry| entry.sample.as_ref())
    }

    /// Persist a spent attempt before invoking management; lost responses remain spent.
    pub fn reserve(&mut self, canister: Principal) -> Result<(), CapacityImportJournalError> {
        let entry = self
            .record
            .canisters
            .get_mut(&canister.to_text())
            .ok_or(CapacityImportJournalError::Integrity)?;
        if entry.sample.is_some() {
            return Err(CapacityImportJournalError::Integrity);
        }
        if entry.attempts >= MAXIMUM_ATTEMPTS {
            return Err(CapacityImportJournalError::BudgetExhausted {
                step: format!("initial survey of {canister}"),
            });
        }
        entry.attempts += 1;
        self.save()
    }

    /// Freeze a successful authenticated sample under its already reserved attempt.
    pub fn retain(
        &mut self,
        sample: CapacityImportSampleRecord,
    ) -> Result<(), CapacityImportJournalError> {
        let id = sample.binding.canister_id.to_text();
        let entry = self
            .record
            .canisters
            .get_mut(&id)
            .ok_or(CapacityImportJournalError::Integrity)?;
        if entry.attempts == 0
            || entry
                .sample
                .as_ref()
                .is_some_and(|previous| previous != &sample)
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        entry.sample = Some(sample);
        self.save()
    }

    fn save(&self) -> Result<(), CapacityImportJournalError> {
        let bytes = serde_json::to_vec(&self.record)?;
        if bytes.len() > MAXIMUM_BYTES {
            return Err(CapacityImportJournalError::Integrity);
        }
        write_bytes(&self.path, &bytes)?;
        Ok(())
    }
}

fn validate(
    record: &CapacityImportSurveyRecord,
    digest: [u8; 32],
    keys: &BTreeSet<String>,
) -> Result<(), CapacityImportJournalError> {
    if record.schema_version != 1
        || record.request_sha256 != digest
        || record.canisters.keys().cloned().collect::<BTreeSet<_>>() != *keys
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    for (id, entry) in &record.canisters {
        if entry.attempts > MAXIMUM_ATTEMPTS
            || entry.sample.as_ref().is_some_and(|sample| {
                entry.attempts == 0 || sample.binding.canister_id.to_text() != *id
            })
        {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    Ok(())
}
