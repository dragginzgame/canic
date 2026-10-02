//! Validate finite continuation grants without changing spent counters or original authority.

use crate::fleet_ensure::{
    model::attempt_recovery::AttemptRecoveryGrantRecord,
    ops::capacity_import::journal::CapacityImportJournalError,
};
use sha2_host::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::fleet_ensure) const ADDITIONAL_ATTEMPTS: u32 = 2;

pub(in crate::fleet_ensure) fn maximum(
    grants: &[AttemptRecoveryGrantRecord],
    binding: [u8; 32],
    resource: &str,
    initial: u32,
) -> Result<u32, CapacityImportJournalError> {
    let mut seen = BTreeSet::new();
    let mut ceilings = BTreeMap::<&str, u32>::new();
    for grant in grants {
        if grant.binding_sha256 != binding
            || grant.resource.is_empty()
            || grant.additional_attempts != ADDITIONAL_ATTEMPTS
            || grant.spent_attempts != grant.previous_maximum
            || grant.previous_maximum == 0
            || grant.grant_sha256 != digest(grant)?
            || !seen.insert(grant.grant_sha256)
            || ceilings
                .get(grant.resource.as_str())
                .is_some_and(|previous| grant.previous_maximum < *previous)
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        let ceiling = grant
            .previous_maximum
            .checked_add(grant.additional_attempts)
            .ok_or(CapacityImportJournalError::Integrity)?;
        ceilings.insert(&grant.resource, ceiling);
    }
    Ok(initial.max(ceilings.get(resource).copied().unwrap_or(0)))
}

pub(super) fn grant(
    binding: [u8; 32],
    resource: String,
    spent: u32,
    maximum: u32,
) -> Result<AttemptRecoveryGrantRecord, CapacityImportJournalError> {
    if spent != maximum {
        return Err(CapacityImportJournalError::Integrity);
    }
    let mut grant = AttemptRecoveryGrantRecord {
        binding_sha256: binding,
        resource,
        spent_attempts: spent,
        previous_maximum: maximum,
        additional_attempts: ADDITIONAL_ATTEMPTS,
        grant_sha256: [0; 32],
    };
    grant.grant_sha256 = digest(&grant)?;
    Ok(grant)
}

fn digest(grant: &AttemptRecoveryGrantRecord) -> Result<[u8; 32], CapacityImportJournalError> {
    let mut grant = grant.clone();
    grant.grant_sha256 = [0; 32];
    let mut hash = Sha256::new();
    hash.update(b"canic:host-attempt-continuation:v1\0");
    hash.update(serde_json::to_vec(&grant)?);
    Ok(hash.finalize().into())
}
