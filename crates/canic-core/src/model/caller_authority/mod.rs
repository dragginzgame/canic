//! Module: model::caller_authority
//!
//! Receiver publication records and exact single-operation transition invariants.
//!
//! Storage indexes independent rows; a transition never clones the receiver census.

use crate::ids::{
    CallerComponentInstallation, CallerInstallation, CallerReceiverAuthority, ComponentInstanceId,
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

///
/// CallerChangeRecord
///
/// One source grant or irrevocable installation-bound denial.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerChangeRecord {
    StageSource(CallerInstallation),
    Grant(CallerInstallation),
    DenySource(CallerInstallation),
    DenyComponent(CallerComponentInstallation),
    OpenReceiver,
    RetireReceiver,
}

///
/// CallerPublicationRecord
///
/// Original publication identity; retries must retain every field.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerPublicationRecord {
    pub operation_id: [u8; 32],
    pub authority: CallerReceiverAuthority,
    pub previous_generation: u64,
    pub generation: u64,
    pub change: CallerChangeRecord,
    pub content_hash: [u8; 32],
}

///
/// CallerReceiptPhase
///
/// Durable recipient receipt phases for prepare, commit and completion reconciliation.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CallerReceiptPhase {
    Prepared,
    Committed,
    Complete,
}

///
/// CallerReceiptRecord
///
/// An independently indexed original-operation receipt.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerReceiptRecord {
    pub publication: CallerPublicationRecord,
    pub phase: CallerReceiptPhase,
}

///
/// CallerReceiverRecord
///
/// Fixed-size control header; entries and receipts are stored in separate indexed rows.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerReceiverRecord {
    pub authority: CallerReceiverAuthority,
    pub generation: u64,
    pub open: bool,
    pub retired: bool,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    pub pending_operation: Option<[u8; 32]>,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    pub cleanup_cursor: Option<CallerRowKey>,
    pub entries: u32,
    pub reserved_bytes: u32,
}

///
/// CallerSourceRecord
///
/// Exact durable source identity and its currently admitted generation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerSourceRecord {
    pub installation: CallerInstallation,
    pub generation: u64,
    pub open: bool,
    pub denied: bool,
}

///
/// CallerPublicationError
///
/// Refusals preserve publication identity, ordering and pre-effect capacity reservation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum CallerPublicationError {
    #[error("caller publication authority does not match this installation")]
    AuthorityConflict,

    #[error("caller publication capacity cannot cover the complete reservation")]
    Capacity,

    #[error("another caller publication remains unfinished")]
    InProgress,

    #[error("caller publication identity or digest is invalid")]
    InvalidPublication,

    #[error("caller publication generation does not follow the retained authority")]
    GenerationConflict,

    #[error("caller publication receipt does not cover the requested phase")]
    PhaseConflict,

    #[error("caller publication replay differs from its original operation")]
    ReplayConflict,

    #[error("retired receiver or source cannot be reopened")]
    Retired,
}

impl CallerReceiverRecord {
    /// Prepare a receiver header after its protected installation identity is validated.
    #[must_use]
    pub const fn new(authority: CallerReceiverAuthority) -> Self {
        Self {
            authority,
            generation: 0,
            open: false,
            retired: false,
            pending_operation: None,
            cleanup_cursor: None,
            entries: 0,
            reserved_bytes: crate::config::caller_authority::CALLER_HEADER_BYTES,
        }
    }

    /// Reserve the complete operation before its first denial or remote effect.
    pub fn reserve(
        &self,
        publication: &CallerPublicationRecord,
        reservation: CallerReservation,
    ) -> Result<Self, CallerPublicationError> {
        self.validate_publication(publication)?;
        if self.pending_operation.is_some() {
            return Err(CallerPublicationError::InProgress);
        }
        let entries = self
            .entries
            .checked_add(reservation.entries)
            .filter(|entries| *entries <= reservation.maximum_entries)
            .ok_or(CallerPublicationError::Capacity)?;
        let reserved_bytes = self
            .reserved_bytes
            .checked_add(reservation.bytes)
            .filter(|bytes| *bytes <= reservation.maximum_bytes)
            .ok_or(CallerPublicationError::Capacity)?;
        let mut successor = self.clone();
        successor.entries = entries;
        successor.reserved_bytes = reserved_bytes;
        successor.pending_operation = Some(publication.operation_id);
        successor.cleanup_cursor = None;
        if matches!(publication.change, CallerChangeRecord::RetireReceiver) {
            successor.open = false;
        }
        Ok(successor)
    }

    /// Advance only the exact pending receipt; old terminal receipts never change the header.
    pub fn finish(&self, receipt: &CallerReceiptRecord) -> Result<Self, CallerPublicationError> {
        if receipt.phase != CallerReceiptPhase::Committed {
            return Err(CallerPublicationError::PhaseConflict);
        }
        self.validate_publication(&receipt.publication)?;
        if self.pending_operation != Some(receipt.publication.operation_id) {
            return Err(CallerPublicationError::ReplayConflict);
        }
        let mut successor = self.clone();
        successor.generation = receipt.publication.generation;
        successor.pending_operation = None;
        successor.cleanup_cursor = None;
        match receipt.publication.change {
            CallerChangeRecord::OpenReceiver => successor.open = true,
            CallerChangeRecord::RetireReceiver => {
                successor.open = false;
                successor.retired = true;
            }
            CallerChangeRecord::StageSource(_)
            | CallerChangeRecord::Grant(_)
            | CallerChangeRecord::DenySource(_)
            | CallerChangeRecord::DenyComponent(_) => {}
        }
        Ok(successor)
    }

    fn validate_publication(
        &self,
        publication: &CallerPublicationRecord,
    ) -> Result<(), CallerPublicationError> {
        if self.retired {
            return Err(CallerPublicationError::Retired);
        }
        if self.authority != publication.authority {
            return Err(CallerPublicationError::AuthorityConflict);
        }
        if publication.operation_id == [0; 32] || publication.content_hash == [0; 32] {
            return Err(CallerPublicationError::InvalidPublication);
        }
        if publication.previous_generation != self.generation
            || self.generation.checked_add(1) != Some(publication.generation)
        {
            return Err(CallerPublicationError::GenerationConflict);
        }
        Ok(())
    }
}

/// Complete encoded-record reservation computed before a transition is accepted.
pub struct CallerReservation {
    pub entries: u32,
    pub bytes: u32,
    pub maximum_entries: u32,
    pub maximum_bytes: u32,
}

impl CallerReceiptRecord {
    /// Exact replay can only advance a retained receipt by one phase.
    pub fn advance(
        &self,
        publication: &CallerPublicationRecord,
        requested: CallerReceiptPhase,
    ) -> Result<Self, CallerPublicationError> {
        if self.publication != *publication {
            return Err(CallerPublicationError::ReplayConflict);
        }
        let permitted = !matches!(
            (self.phase, requested),
            (CallerReceiptPhase::Prepared, CallerReceiptPhase::Complete)
        );
        if !permitted {
            return Err(CallerPublicationError::PhaseConflict);
        }
        let phase = match (self.phase, requested) {
            (CallerReceiptPhase::Complete, _) => CallerReceiptPhase::Complete,
            (CallerReceiptPhase::Committed, CallerReceiptPhase::Prepared) => {
                CallerReceiptPhase::Committed
            }
            (_, requested) => requested,
        };
        Ok(Self {
            publication: self.publication.clone(),
            phase,
        })
    }
}

///
/// CallerRowKey
///
/// Tagged stable row key; Principal, Component or original operation identity.
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CallerRowKey(Vec<u8>);

impl CallerRowKey {
    pub(crate) fn source(caller: Principal) -> Self {
        Self::tagged(0, caller.as_slice())
    }
    pub(crate) fn component(component: ComponentInstanceId, installation: [u8; 32]) -> Self {
        let mut key = Self::tagged(1, component.as_bytes());
        key.0.extend_from_slice(&installation);
        key
    }
    pub(crate) fn receipt(operation: [u8; 32]) -> Self {
        Self::tagged(2, &operation)
    }

    fn tagged(tag: u8, identity: &[u8]) -> Self {
        let mut key = Vec::with_capacity(identity.len() + 1);
        key.push(tag);
        key.extend_from_slice(identity);
        Self(key)
    }
}
