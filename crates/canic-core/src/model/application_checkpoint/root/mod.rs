//! Module: model::application_checkpoint::root
//!
//! Responsibility: retain Root-issued unit identity and participant acknowledgements.
//! Does not own: discovery, stable memory, hashing, paid effects or application hooks.
//! Boundary: ops commits a complete replacement before workflow sends participant calls.

#[cfg(test)]
mod tests;

use super::{ApplicationCheckpointError, validate_binding};
use canic_contracts::ids::{
    ApplicationCheckpointBinding, ApplicationCheckpointPhase, CallerReceiverAuthority,
    CallerRootAuthority,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bounded participant catalogue for one Root-coordinated application unit.
pub const MAX_CHECKPOINT_PARTICIPANTS: usize = 4096;

/// Root acknowledgement phase; completion requires every selected participant.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootCheckpointPhase {
    Preparing,
    Prepared,
    Releasing,
    Released,
}

/// One exact participant and its latest acknowledged Root-directed phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootCheckpointParticipantRecord {
    binding: ApplicationCheckpointBinding,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    acknowledged: Option<ApplicationCheckpointPhase>,
}

/// Complete original unit retained outside application snapshots.
///
/// This journal grants no snapshot or spending authority. The workflow must hold
/// its separate paid-effect custody until every unknown snapshot effect is resolved.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RootCheckpointFields")]
pub struct RootCheckpointRecord {
    participants: Vec<RootCheckpointParticipantRecord>,
    phase: RootCheckpointPhase,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootCheckpointFields {
    participants: Vec<RootCheckpointParticipantRecord>,
    phase: RootCheckpointPhase,
}

/// Refusal leaves the original unit, sequence and all acknowledgements unchanged.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RootCheckpointError {
    #[error("checkpoint participant identity is invalid")]
    InvalidParticipant,
    #[error("checkpoint participant catalogue is empty, duplicated or too large")]
    InvalidCatalogue,
    #[error("checkpoint original unit or Root authority differs")]
    IdentityConflict,
    #[error("checkpoint issuance sequence is stale or exhausted")]
    InvalidSequence,
    #[error("another checkpoint remains active")]
    ActiveCheckpoint,
    #[error("participant acknowledgement does not match the Root phase")]
    InvalidAcknowledgement,
    #[error("checkpoint record has inconsistent retained progress")]
    InvalidRecord,
}

impl TryFrom<RootCheckpointFields> for RootCheckpointRecord {
    type Error = RootCheckpointError;

    fn try_from(fields: RootCheckpointFields) -> Result<Self, Self::Error> {
        let record = Self {
            participants: fields.participants,
            phase: fields.phase,
        };
        record.validate()?;
        Ok(record)
    }
}

impl RootCheckpointRecord {
    /// Issue a successor or replay the exact original request without incrementing.
    ///
    /// `expected_sequence` is the host's retained pre-issuance sequence. It prevents
    /// a delayed original begin from creating a fresh unit after another operation.
    /// Discovery supplies the complete current catalogue and its canonical digest.
    pub fn begin(
        previous: Option<&Self>,
        root: &CallerRootAuthority,
        expected_sequence: u64,
        operation_id: [u8; 32],
        unit_digest: [u8; 32],
        mut authorities: Vec<CallerReceiverAuthority>,
    ) -> Result<Self, RootCheckpointError> {
        if authorities.is_empty() || authorities.len() > MAX_CHECKPOINT_PARTICIPANTS {
            return Err(RootCheckpointError::InvalidCatalogue);
        }
        authorities.sort_by_key(|authority| authority.receiver.canister());
        let sequence = expected_sequence
            .checked_add(1)
            .ok_or(RootCheckpointError::InvalidSequence)?;
        let candidate = Self {
            participants: authorities
                .into_iter()
                .map(|authority| RootCheckpointParticipantRecord {
                    binding: ApplicationCheckpointBinding {
                        authority,
                        operation_id,
                        sequence,
                        unit_digest,
                    },
                    acknowledged: None,
                })
                .collect(),
            phase: RootCheckpointPhase::Preparing,
        };
        candidate.validate()?;
        if candidate.identity().authority.issuer != *root {
            return Err(RootCheckpointError::IdentityConflict);
        }
        if let Some(previous) = previous {
            previous.validate()?;
            if previous.identity().authority.issuer != *root {
                return Err(RootCheckpointError::IdentityConflict);
            }
            if previous.identity().operation_id == operation_id {
                if !previous.same_unit(&candidate) {
                    return Err(RootCheckpointError::IdentityConflict);
                }
                return Ok(previous.clone());
            }
            if previous.identity().sequence != expected_sequence {
                return Err(RootCheckpointError::InvalidSequence);
            }
            if previous.phase != RootCheckpointPhase::Released {
                return Err(RootCheckpointError::ActiveCheckpoint);
            }
        } else if expected_sequence != 0 {
            return Err(RootCheckpointError::InvalidSequence);
        }
        Ok(candidate)
    }

    /// Inspect exact immutable participant identities and retained acknowledgements.
    #[must_use]
    pub fn participants(&self) -> &[RootCheckpointParticipantRecord] {
        &self.participants
    }

    /// Inspect aggregate progress without inferring IC snapshot completion.
    #[must_use]
    pub const fn phase(&self) -> RootCheckpointPhase {
        self.phase
    }

    /// Catalogue mutation stays excluded until every participant acknowledges release.
    #[must_use]
    pub fn membership_mutation_allowed(&self) -> bool {
        self.phase == RootCheckpointPhase::Released
    }

    /// Retain release intent for every participant, including unanswered prepares.
    ///
    /// Snapshot reconciliation remains a prerequisite owned by the calling workflow.
    #[must_use]
    pub fn release(&self) -> Self {
        if matches!(
            self.phase,
            RootCheckpointPhase::Releasing | RootCheckpointPhase::Released
        ) {
            return self.clone();
        }
        let mut next = self.clone();
        next.phase = RootCheckpointPhase::Releasing;
        next
    }

    /// Retain an exact participant acknowledgement; repeated replies are effect-free.
    pub fn acknowledge(
        &self,
        binding: &ApplicationCheckpointBinding,
        phase: ApplicationCheckpointPhase,
    ) -> Result<Self, RootCheckpointError> {
        let expected = match self.phase {
            RootCheckpointPhase::Preparing | RootCheckpointPhase::Prepared => {
                ApplicationCheckpointPhase::Prepared
            }
            RootCheckpointPhase::Releasing | RootCheckpointPhase::Released => {
                ApplicationCheckpointPhase::Released
            }
        };
        if phase != expected {
            return Err(RootCheckpointError::InvalidAcknowledgement);
        }
        let mut next = self.clone();
        let participant = next
            .participants
            .iter_mut()
            .find(|participant| participant.binding == *binding)
            .ok_or(RootCheckpointError::IdentityConflict)?;
        participant.acknowledged = Some(phase);
        if next
            .participants
            .iter()
            .all(|participant| participant.acknowledged == Some(expected))
        {
            next.phase = match expected {
                ApplicationCheckpointPhase::Prepared => RootCheckpointPhase::Prepared,
                ApplicationCheckpointPhase::Released => RootCheckpointPhase::Released,
                _ => unreachable!("Root only accepts terminal hook acknowledgements"),
            };
        }
        Ok(next)
    }

    fn identity(&self) -> &ApplicationCheckpointBinding {
        &self.participants[0].binding
    }

    fn same_unit(&self, other: &Self) -> bool {
        self.participants
            .iter()
            .map(|participant| &participant.binding)
            .eq(other
                .participants
                .iter()
                .map(|participant| &participant.binding))
    }

    fn validate(&self) -> Result<(), RootCheckpointError> {
        if self.participants.is_empty() || self.participants.len() > MAX_CHECKPOINT_PARTICIPANTS {
            return Err(RootCheckpointError::InvalidCatalogue);
        }
        let first = self.identity();
        let mut previous = None;
        for participant in &self.participants {
            let binding = &participant.binding;
            validate_binding(binding)
                .map_err(|_: ApplicationCheckpointError| RootCheckpointError::InvalidParticipant)?;
            let canister = binding.authority.receiver.canister();
            if previous.is_some_and(|previous| previous >= canister) {
                return Err(RootCheckpointError::InvalidCatalogue);
            }
            previous = Some(canister);
            let same_checkpoint = (
                &binding.authority.issuer,
                binding.operation_id,
                binding.sequence,
                binding.unit_digest,
            ) == (
                &first.authority.issuer,
                first.operation_id,
                first.sequence,
                first.unit_digest,
            );
            if !same_checkpoint {
                return Err(RootCheckpointError::IdentityConflict);
            }
            let allowed = match self.phase {
                RootCheckpointPhase::Preparing => matches!(
                    participant.acknowledged,
                    None | Some(ApplicationCheckpointPhase::Prepared)
                ),
                RootCheckpointPhase::Prepared => {
                    participant.acknowledged == Some(ApplicationCheckpointPhase::Prepared)
                }
                RootCheckpointPhase::Releasing => matches!(
                    participant.acknowledged,
                    None | Some(
                        ApplicationCheckpointPhase::Prepared | ApplicationCheckpointPhase::Released
                    )
                ),
                RootCheckpointPhase::Released => {
                    participant.acknowledged == Some(ApplicationCheckpointPhase::Released)
                }
            };
            if !allowed {
                return Err(RootCheckpointError::InvalidRecord);
            }
        }
        Ok(())
    }
}

impl RootCheckpointParticipantRecord {
    /// Exact installed audience to use for the next Root call.
    #[must_use]
    pub const fn binding(&self) -> &ApplicationCheckpointBinding {
        &self.binding
    }

    /// Latest retained acknowledgement; absence does not prove nonapplication.
    #[must_use]
    pub const fn acknowledged(&self) -> Option<ApplicationCheckpointPhase> {
        self.acknowledged
    }
}
