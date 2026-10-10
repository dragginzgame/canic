//! Durable release read allowances in the existing Fleet journal and operation lock.
//!
//! Reservations are never refunded. A persistence error prevents issuing a token and
//! poisons this owner until its exact durable state is reopened.

#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

use crate::fleet_ensure::{
    model::{
        FleetEnsureCompletion, FleetEnsureJournalRecord,
        release::{FleetReleaseAuthority, FleetReleaseExecutionRecord, FleetReleaseReviewRecord},
    },
    ops::{EnsurePaths, EnsureStateError, read_journal, write_journal},
    policy::release::{FleetReleaseError, reserve_observation_calls},
    view::release::FleetReleaseObservation,
};
use candid::Principal;
use canic_contracts::ids::SubnetId;
use canic_core::cdk::utils::hash::hex_bytes;
use std::{collections::BTreeMap, fs::File};
use thiserror::Error;

/// Local refusal occurs before issuing any paid observation.
#[derive(Debug, Error)]
pub enum ReleaseReservationError {
    #[error(transparent)]
    State(#[from] EnsureStateError),
    #[error(transparent)]
    Evidence(#[from] FleetReleaseError),
    #[error("release journal, review or operation authority differs")]
    Integrity,
    #[error("release reservation persistence was uncertain; reopen the retained journal")]
    PersistenceUncertain,
}

/// Holds the ordinary Fleet lock across reservation persistence and its paid reads.
pub struct ReleaseObservationJournal {
    _lock: File,
    paths: EnsurePaths,
    journal: FleetEnsureJournalRecord,
    poisoned: bool,
}

/// One non-cloneable allowance, keeping its exact journal owner alive through observation.
pub struct ReleaseObservationReservation<'a> {
    owner: &'a ReleaseObservationJournal,
    canister: Principal,
    subnet: SubnetId,
}

impl ReleaseObservationReservation<'_> {
    pub(super) fn authorizes(
        &self,
        authority: &FleetReleaseAuthority,
        canister: Principal,
        subnet: SubnetId,
    ) -> bool {
        self.canister == canister
            && self.subnet == subnet
            && self
                .owner
                .journal
                .release
                .as_ref()
                .is_some_and(|record| &record.review.authority == authority)
    }
}

impl ReleaseObservationJournal {
    /// Attach admitted release authority to an already retained, matching operation envelope.
    /// This does not create an operation, authorize destructive effects or collect live evidence.
    pub fn attach(
        paths: &EnsurePaths,
        review: &FleetReleaseReviewRecord,
        observed: &FleetReleaseObservation,
    ) -> Result<Self, ReleaseReservationError> {
        super::verify_review(review, observed)?;
        let mut owner = Self::read_locked(paths)?;
        validate_envelope(&owner.journal, review)?;
        if owner.journal.release.is_none() {
            if !owner.journal.effects.is_empty() {
                return Err(ReleaseReservationError::Integrity);
            }
            owner.journal.release = Some(FleetReleaseExecutionRecord {
                plan_sha256: owner.journal.plan_sha256.clone(),
                review: review.clone(),
                reserved_paid_calls: review
                    .sources
                    .iter()
                    .map(|source| (source.binding.canister_id.to_text(), 0))
                    .collect(),
            });
            owner.save()?;
        }
        owner.validate(review)?;
        Ok(owner)
    }

    /// Resume exact retained authority without rebasing balances or spending a live read.
    pub fn resume(
        paths: &EnsurePaths,
        review: &FleetReleaseReviewRecord,
    ) -> Result<Self, ReleaseReservationError> {
        let owner = Self::read_locked(paths)?;
        owner.validate(review)?;
        Ok(owner)
    }

    /// Charge all four reads before returning the only token accepted by physical observation.
    pub fn reserve(
        &mut self,
        canister: Principal,
    ) -> Result<ReleaseObservationReservation<'_>, ReleaseReservationError> {
        if self.poisoned {
            return Err(ReleaseReservationError::PersistenceUncertain);
        }
        if read_journal(&self.paths)?.as_ref() != Some(&self.journal) {
            return Err(ReleaseReservationError::Integrity);
        }
        let record = self
            .journal
            .release
            .as_mut()
            .ok_or(ReleaseReservationError::Integrity)?;
        let source = record
            .review
            .sources
            .iter()
            .find(|source| source.binding.canister_id == canister)
            .ok_or(ReleaseReservationError::Integrity)?;
        let calls = record
            .reserved_paid_calls
            .get_mut(&canister.to_text())
            .ok_or(ReleaseReservationError::Integrity)?;
        *calls = reserve_observation_calls(
            source,
            *calls,
            super::observation::PHYSICAL_SAMPLE_PAID_CALLS,
        )?;
        let subnet = source.binding.subnet;
        self.save()?;
        Ok(ReleaseObservationReservation {
            owner: self,
            canister,
            subnet,
        })
    }

    fn read_locked(paths: &EnsurePaths) -> Result<Self, ReleaseReservationError> {
        let lock = crate::fleet_ensure::ops::lock_fleet_file(paths)?;
        crate::fleet_ensure::ops::capacity_import::journal::require_no_approved_import(paths)?;
        let journal = read_journal(paths)?.ok_or(ReleaseReservationError::Integrity)?;
        Ok(Self {
            _lock: lock,
            paths: paths.clone(),
            journal,
            poisoned: false,
        })
    }

    fn validate(&self, review: &FleetReleaseReviewRecord) -> Result<(), ReleaseReservationError> {
        super::verify_digest(review)?;
        validate_envelope(&self.journal, review)?;
        let record = self
            .journal
            .release
            .as_ref()
            .ok_or(ReleaseReservationError::Integrity)?;
        let sources = review
            .sources
            .iter()
            .map(|source| (source.binding.canister_id.to_text(), source))
            .collect::<BTreeMap<_, _>>();
        if &record.review != review
            || record.plan_sha256 != self.journal.plan_sha256
            || sources.len() != review.sources.len()
            || sources.len() != record.reserved_paid_calls.len()
        {
            return Err(ReleaseReservationError::Integrity);
        }
        for (id, calls) in &record.reserved_paid_calls {
            let source = sources.get(id).ok_or(ReleaseReservationError::Integrity)?;
            reserve_observation_calls(source, *calls, 0)?;
        }
        Ok(())
    }

    fn save(&mut self) -> Result<(), ReleaseReservationError> {
        if let Err(error) = write_journal(&self.paths, &self.journal) {
            self.poisoned = true;
            return Err(error.into());
        }
        Ok(())
    }
}

fn validate_envelope(
    journal: &FleetEnsureJournalRecord,
    review: &FleetReleaseReviewRecord,
) -> Result<(), ReleaseReservationError> {
    let operation_matches = journal.operation_id == hex_bytes(review.authority.operation_id)
        && crate::fleet_ensure::ops::is_sha256(&journal.plan_sha256)
        && journal.completion == FleetEnsureCompletion::InProgress;
    let independent_authority = journal.bootstrap_registration_recovery.is_some()
        || journal.estate_funding_required.is_some();
    let ordinary_history_empty = journal.funding_reviews.is_empty()
        && journal.funding_observations.is_empty()
        && journal.successor_phases.is_empty();
    if !operation_matches || independent_authority || !ordinary_history_empty {
        return Err(ReleaseReservationError::Integrity);
    }
    Ok(())
}

/// Ordinary Ensure/import cannot take over an unfinished release operation.
pub(in crate::fleet_ensure::ops) fn require_no_active_release(
    paths: &EnsurePaths,
) -> Result<(), EnsureStateError> {
    if let Some(journal) = crate::fleet_ensure::ops::operation_selection::read(&paths.journal)?
        && journal
            .get("release")
            .is_some_and(|release| !release.is_null())
        && journal
            .get("completion")
            .and_then(serde_json::Value::as_str)
            != Some("converged")
    {
        return Err(EnsureStateError::ReleaseInProgress {
            path: paths.journal.clone(),
        });
    }
    Ok(())
}
