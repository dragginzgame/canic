//! Typed retained-counter owners for exact, bounded local continuation grants.

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        model::{
            attempt_recovery::{AttemptRecoveryGrantRecord, AttemptRecoveryOwnerRecord},
            capacity_import::{CapacityImportJournalRecord, survey::CapacityImportSurveyRecord},
            infrastructure_bootstrap::{
                BOOTSTRAP_EFFECT_INSPECTION_ROUNDS, BOOTSTRAP_PHASE_INSPECTION_ROUNDS,
                InfrastructureBootstrapInspectionRecord,
                registration_recovery::RECOVERY_INSPECTION_ROUNDS,
            },
        },
        ops::{
            EnsurePaths,
            attempt_recovery::allowance,
            capacity_import::{
                admission::survey::{self, MAXIMUM_ATTEMPTS},
                journal::{self, CapacityImportJournalError},
                publication,
            },
            infrastructure_bootstrap::inspection,
        },
    },
};
use canic_core::cdk::utils::hash::hex_bytes;
use sha2_host::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_OWNERS: usize = 4096;

enum Owner {
    Bootstrap(InfrastructureBootstrapInspectionRecord),
    Import(Box<CapacityImportJournalRecord>),
    Survey(CapacityImportSurveyRecord),
}

struct Counter {
    resource: String,
    spent: u32,
    maximum: u32,
}

pub(super) fn discover(
    paths: &EnsurePaths,
) -> Result<Vec<AttemptRecoveryOwnerRecord>, CapacityImportJournalError> {
    let directory = paths
        .plan
        .parent()
        .ok_or(CapacityImportJournalError::Integrity)?;
    let mut names = Vec::new();
    if directory.join("capacity-import.json").exists() {
        names.push("capacity-import.json".to_owned());
    }
    for name in [
        "capacity-import-surveys",
        "infrastructure-bootstrap-inspections",
        "infrastructure-bootstrap-surveys",
    ] {
        let path = directory.join(name);
        if !path.exists() {
            continue;
        }
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(CapacityImportJournalError::Integrity);
        }
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || entry
                    .path()
                    .extension()
                    .is_none_or(|extension| extension != "json")
            {
                return Err(CapacityImportJournalError::Integrity);
            }
            let filename = entry
                .file_name()
                .into_string()
                .map_err(|_| CapacityImportJournalError::Integrity)?;
            names.push(format!("{name}/{filename}"));
            if names.len() > MAX_OWNERS {
                return Err(CapacityImportJournalError::Integrity);
            }
        }
    }
    names.sort();
    let mut result = Vec::new();
    for relative in names {
        let (bytes, owner) = read(paths, &relative)?;
        let grants = owner.exhausted()?;
        if !grants.is_empty() {
            result.push(AttemptRecoveryOwnerRecord {
                relative_path: relative,
                before_sha256: Sha256::digest(bytes).into(),
                grants,
            });
        }
    }
    Ok(result)
}

pub(super) fn prepare(
    paths: &EnsurePaths,
    reviewed: &AttemptRecoveryOwnerRecord,
) -> Result<Option<(PathBuf, Vec<u8>)>, CapacityImportJournalError> {
    let (bytes, mut owner) = read(paths, &reviewed.relative_path)?;
    if reviewed.grants.is_empty() {
        return Err(CapacityImportJournalError::Integrity);
    }
    if reviewed
        .grants
        .iter()
        .all(|grant| owner.grants().contains(grant))
    {
        return Ok(None);
    }
    if Sha256::digest(&bytes).as_slice() != reviewed.before_sha256
        || owner.exhausted()? != reviewed.grants
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    owner.grants_mut()?.extend(reviewed.grants.clone());
    owner.validate()?;
    if let Owner::Import(record) = &owner {
        journal::require_completion_fits(record)?;
    }
    let maximum = owner.maximum_bytes();
    let bytes = match owner {
        Owner::Bootstrap(record) => serde_json::to_vec(&record)?,
        Owner::Import(record) => serde_json::to_vec_pretty(&record)?,
        Owner::Survey(record) => serde_json::to_vec(&record)?,
    };
    if bytes.len() > maximum {
        return Err(CapacityImportJournalError::Integrity);
    }
    Ok(Some((path(paths, &reviewed.relative_path)?, bytes)))
}

fn path(paths: &EnsurePaths, relative: &str) -> Result<PathBuf, CapacityImportJournalError> {
    let relative = Path::new(relative);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    let path = paths
        .plan
        .parent()
        .ok_or(CapacityImportJournalError::Integrity)?
        .join(relative);
    if path.canonicalize()? != path {
        return Err(CapacityImportJournalError::Integrity);
    }
    Ok(path)
}

fn read(
    paths: &EnsurePaths,
    relative: &str,
) -> Result<(Vec<u8>, Owner), CapacityImportJournalError> {
    let path = path(paths, relative)?;
    let bytes = read_regular_bytes(&path, MAX_BYTES)?;
    let owner = if relative == "capacity-import.json" {
        Owner::Import(Box::new(serde_json::from_slice(&bytes)?))
    } else if relative.starts_with("infrastructure-bootstrap-inspections/") {
        Owner::Bootstrap(serde_json::from_slice(&bytes)?)
    } else if relative.starts_with("capacity-import-surveys/")
        || relative.starts_with("infrastructure-bootstrap-surveys/")
    {
        Owner::Survey(serde_json::from_slice(&bytes)?)
    } else {
        return Err(CapacityImportJournalError::Integrity);
    };
    if !matches!(owner, Owner::Import(_))
        && path.file_stem().and_then(|stem| stem.to_str())
            != Some(hex_bytes(owner.binding()).as_str())
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    owner.validate()?;
    if bytes.len() > owner.maximum_bytes() {
        return Err(CapacityImportJournalError::Integrity);
    }
    Ok((bytes, owner))
}

impl Owner {
    const fn maximum_bytes(&self) -> usize {
        match self {
            Self::Bootstrap(_) => inspection::MAXIMUM_BYTES,
            Self::Import(_) => MAX_BYTES,
            Self::Survey(_) => survey::MAXIMUM_BYTES,
        }
    }

    fn binding(&self) -> [u8; 32] {
        match self {
            Self::Bootstrap(record) => record.source_sha256,
            Self::Import(record) => record
                .operation
                .as_ref()
                .map_or([0; 32], |operation| operation.review.review_sha256),
            Self::Survey(record) => record.request_sha256,
        }
    }

    fn grants(&self) -> &[AttemptRecoveryGrantRecord] {
        match self {
            Self::Bootstrap(record) => &record.attempt_recoveries,
            Self::Import(record) => record.operation.as_ref().map_or(&[] as &[_], |operation| {
                operation.attempt_recoveries.as_slice()
            }),
            Self::Survey(record) => &record.attempt_recoveries,
        }
    }

    fn grants_mut(
        &mut self,
    ) -> Result<&mut Vec<AttemptRecoveryGrantRecord>, CapacityImportJournalError> {
        Ok(match self {
            Self::Bootstrap(record) => &mut record.attempt_recoveries,
            Self::Import(record) => {
                &mut record
                    .operation
                    .as_mut()
                    .ok_or(CapacityImportJournalError::Integrity)?
                    .attempt_recoveries
            }
            Self::Survey(record) => &mut record.attempt_recoveries,
        })
    }

    fn counters(&self) -> Result<Vec<Counter>, CapacityImportJournalError> {
        let mut raw = Vec::new();
        match self {
            Self::Survey(record) => {
                for (id, entry) in &record.canisters {
                    raw.push((id.clone(), entry.attempts, MAXIMUM_ATTEMPTS));
                }
            }
            Self::Import(record) => {
                if let Some(operation) = &record.operation {
                    for (key, spent) in &operation.submissions {
                        raw.push((
                            format!("submission:{key}"),
                            *spent,
                            publication::MAX_SUBMISSIONS,
                        ));
                    }
                    for (key, spent) in &operation.inspections {
                        raw.push((
                            format!("inspection:{key}"),
                            *spent,
                            publication::MAX_INSPECTIONS,
                        ));
                    }
                    for handoff in &record.handoffs {
                        raw.push((
                            format!("handoff_envelopes:{}", handoff.canister_id),
                            u32::try_from(handoff.retirements.len())
                                .map_err(|_| CapacityImportJournalError::Integrity)?,
                            2,
                        ));
                    }
                }
            }
            Self::Bootstrap(record) => {
                let extended = BOOTSTRAP_PHASE_INSPECTION_ROUNDS
                    + if record.registration_recovery_sha256.is_some() {
                        RECOVERY_INSPECTION_ROUNDS
                    } else {
                        0
                    };
                raw.extend([
                    (
                        "review".into(),
                        record.review_attempts,
                        BOOTSTRAP_PHASE_INSPECTION_ROUNDS,
                    ),
                    (
                        "apply".into(),
                        record.apply_attempts,
                        BOOTSTRAP_PHASE_INSPECTION_ROUNDS,
                    ),
                    ("terminal".into(), record.terminal_attempts, extended),
                    (
                        "registration".into(),
                        record.registration_attempts,
                        extended,
                    ),
                ]);
                raw.extend(record.effect_observations.iter().map(|(key, spent)| {
                    (
                        format!("effect:{key}"),
                        *spent,
                        BOOTSTRAP_EFFECT_INSPECTION_ROUNDS,
                    )
                }));
            }
        }
        raw.into_iter()
            .map(|(resource, spent, initial)| {
                let maximum =
                    allowance::maximum(self.grants(), self.binding(), &resource, initial)?;
                Ok(Counter {
                    resource,
                    spent,
                    maximum,
                })
            })
            .collect()
    }

    fn validate(&self) -> Result<(), CapacityImportJournalError> {
        match self {
            Self::Survey(record)
                if record.schema_version != 1
                    || record.request_sha256 == [0; 32]
                    || record.canisters.is_empty() =>
            {
                return Err(CapacityImportJournalError::Integrity);
            }
            Self::Bootstrap(record)
                if record.schema_version != 1 || record.source_sha256 == [0; 32] =>
            {
                return Err(CapacityImportJournalError::Integrity);
            }
            Self::Import(record) => journal::validate(record)?,
            _ => {}
        }
        let counters = self.counters()?;
        if counters
            .iter()
            .any(|counter| counter.spent > counter.maximum)
            || self.grants().iter().any(|grant| {
                counters
                    .iter()
                    .find(|counter| counter.resource == grant.resource)
                    .is_none_or(|counter| counter.spent < grant.spent_attempts)
            })
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        Ok(())
    }

    fn exhausted(&self) -> Result<Vec<AttemptRecoveryGrantRecord>, CapacityImportJournalError> {
        if let Self::Import(record) = self
            && publication::completed(record)
        {
            return Ok(Vec::new());
        }
        self.counters()?
            .into_iter()
            .filter(|counter| {
                if let Self::Survey(record) = self
                    && record.canisters[&counter.resource].sample.is_some()
                {
                    return false;
                }
                counter.spent == counter.maximum
            })
            .map(|counter| {
                allowance::grant(
                    self.binding(),
                    counter.resource,
                    counter.spent,
                    counter.maximum,
                )
            })
            .collect()
    }
}
