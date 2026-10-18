//! Module: model::application_checkpoint
//!
//! Responsibility: own application participant chronology and release invariants.
//! Does not own: storage, hooks, timers, endpoint authentication or IC effects.
//! Boundary: applications persist this record with their business freeze state.

pub mod root;

#[cfg(test)]
pub mod tests;

use canic_contracts::ids::{ApplicationCheckpointBinding, ApplicationCheckpointPhase};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Application-owned high-water evidence, captured with its business snapshot.
///
/// Retaining a Released record prevents a delayed prepare from refreezing state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "CheckpointFields")]
pub struct ApplicationCheckpointRecord {
    binding: ApplicationCheckpointBinding,
    phase: ApplicationCheckpointPhase,
    captured: Option<ApplicationCheckpointBinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointFields {
    binding: ApplicationCheckpointBinding,
    phase: ApplicationCheckpointPhase,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    captured: Option<ApplicationCheckpointBinding>,
}

impl TryFrom<CheckpointFields> for ApplicationCheckpointRecord {
    type Error = ApplicationCheckpointError;

    fn try_from(fields: CheckpointFields) -> Result<Self, Self::Error> {
        validate_binding(&fields.binding)?;
        if let Some(captured) = &fields.captured {
            validate_restore_binding(captured, &fields.binding)?;
        }
        Ok(Self {
            binding: fields.binding,
            phase: fields.phase,
            captured: fields.captured,
        })
    }
}

/// Typed current-contract transition refusal; no error releases application state.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ApplicationCheckpointError {
    #[error("invalid checkpoint identity")]
    InvalidIdentity,
    #[error("checkpoint authority or original identity differs")]
    IdentityConflict,
    #[error("checkpoint sequence is superseded")]
    StaleSequence,
    #[error("another checkpoint remains active")]
    ActiveCheckpoint,
    #[error("checkpoint has already been released")]
    Released,
    #[error("checkpoint phase cannot acknowledge this transition")]
    InvalidPhase,
    #[error("source snapshot does not retain the exact prepared checkpoint")]
    InvalidSource,
}

/// Candidate state change, to commit before invoking an application hook.
pub enum ApplicationCheckpointTransition {
    Changed(ApplicationCheckpointRecord),
    Retained(ApplicationCheckpointRecord),
}

impl ApplicationCheckpointRecord {
    /// Inspect immutable original checkpoint identity.
    #[must_use]
    pub const fn binding(&self) -> &ApplicationCheckpointBinding {
        &self.binding
    }

    /// Inspect the original prepared source retained throughout restoration and release.
    #[must_use]
    pub const fn captured(&self) -> Option<&ApplicationCheckpointBinding> {
        self.captured.as_ref()
    }

    /// Inspect the retained phase without changing application state.
    #[must_use]
    pub const fn phase(&self) -> ApplicationCheckpointPhase {
        self.phase
    }

    /// Application mutation is admissible only after an explicit release acknowledgement.
    #[must_use]
    pub fn mutation_allowed(&self) -> bool {
        self.phase == ApplicationCheckpointPhase::Released
    }

    /// Begin a new freeze or replay its exact retained progress without rerunning hooks.
    pub fn prepare(
        previous: Option<&Self>,
        binding: ApplicationCheckpointBinding,
    ) -> Result<ApplicationCheckpointTransition, ApplicationCheckpointError> {
        validate_binding(&binding)?;
        if let Some(previous) = previous {
            require_current_authority(previous, &binding)?;
            if previous.binding.sequence == binding.sequence {
                require_exact(previous, &binding)?;
                if previous.captured.is_some() {
                    return Err(ApplicationCheckpointError::IdentityConflict);
                }
                return match previous.phase {
                    ApplicationCheckpointPhase::Released => {
                        Err(ApplicationCheckpointError::Released)
                    }
                    ApplicationCheckpointPhase::Releasing => {
                        Err(ApplicationCheckpointError::InvalidPhase)
                    }
                    _ => Ok(ApplicationCheckpointTransition::Retained(previous.clone())),
                };
            }
            if !previous.mutation_allowed() {
                return Err(ApplicationCheckpointError::ActiveCheckpoint);
            }
        }
        Ok(ApplicationCheckpointTransition::Changed(Self {
            binding,
            phase: ApplicationCheckpointPhase::Preparing,
            captured: None,
        }))
    }

    /// Retain release intent even when preparation has not yet arrived.
    pub fn release(
        previous: Option<&Self>,
        binding: ApplicationCheckpointBinding,
    ) -> Result<ApplicationCheckpointTransition, ApplicationCheckpointError> {
        validate_binding(&binding)?;
        if let Some(previous) = previous {
            require_current_authority(previous, &binding)?;
            if previous.binding.sequence == binding.sequence {
                require_exact(previous, &binding)?;
                if matches!(
                    previous.phase,
                    ApplicationCheckpointPhase::Released | ApplicationCheckpointPhase::Releasing
                ) {
                    return Ok(ApplicationCheckpointTransition::Retained(previous.clone()));
                }
            } else if !previous.mutation_allowed() {
                return Err(ApplicationCheckpointError::ActiveCheckpoint);
            }
        }
        let captured = previous
            .filter(|previous| previous.binding == binding)
            .and_then(|previous| previous.captured.clone());
        Ok(ApplicationCheckpointTransition::Changed(Self {
            binding,
            phase: ApplicationCheckpointPhase::Releasing,
            captured,
        }))
    }

    /// Bind an exact prepared source to a fresh same-unit checkpoint after qualified loading.
    ///
    /// Root owns outside-snapshot authority; this structural transition proves no load.
    pub fn restore(
        previous: &Self,
        captured: &ApplicationCheckpointBinding,
        binding: ApplicationCheckpointBinding,
    ) -> Result<ApplicationCheckpointTransition, ApplicationCheckpointError> {
        validate_restore_binding(captured, &binding)?;
        require_current_authority(previous, &binding)?;
        if previous.binding == binding {
            if previous.captured.as_ref() != Some(captured) {
                return Err(ApplicationCheckpointError::IdentityConflict);
            }
            return match previous.phase {
                ApplicationCheckpointPhase::Preparing | ApplicationCheckpointPhase::Prepared => {
                    Ok(ApplicationCheckpointTransition::Retained(previous.clone()))
                }
                ApplicationCheckpointPhase::Releasing => {
                    Err(ApplicationCheckpointError::InvalidPhase)
                }
                ApplicationCheckpointPhase::Released => Err(ApplicationCheckpointError::Released),
            };
        }
        if previous.phase != ApplicationCheckpointPhase::Prepared || &previous.binding != captured {
            return Err(ApplicationCheckpointError::InvalidSource);
        }
        Ok(ApplicationCheckpointTransition::Changed(Self {
            binding,
            phase: ApplicationCheckpointPhase::Preparing,
            captured: Some(captured.clone()),
        }))
    }

    /// Commit application-owned quiescence after every drain/freeze obligation is satisfied.
    pub fn acknowledge_prepared(
        &self,
        binding: &ApplicationCheckpointBinding,
    ) -> Result<Self, ApplicationCheckpointError> {
        self.acknowledge(
            binding,
            ApplicationCheckpointPhase::Preparing,
            ApplicationCheckpointPhase::Prepared,
        )
    }

    /// Commit completed application release; delayed preparation can no longer reopen the fence.
    pub fn acknowledge_released(
        &self,
        binding: &ApplicationCheckpointBinding,
    ) -> Result<Self, ApplicationCheckpointError> {
        self.acknowledge(
            binding,
            ApplicationCheckpointPhase::Releasing,
            ApplicationCheckpointPhase::Released,
        )
    }

    fn acknowledge(
        &self,
        binding: &ApplicationCheckpointBinding,
        from: ApplicationCheckpointPhase,
        to: ApplicationCheckpointPhase,
    ) -> Result<Self, ApplicationCheckpointError> {
        require_exact(self, binding)?;
        if self.phase == to {
            return Ok(self.clone());
        }
        if self.phase != from {
            return Err(ApplicationCheckpointError::InvalidPhase);
        }
        Ok(Self {
            binding: self.binding.clone(),
            phase: to,
            captured: self.captured.clone(),
        })
    }
}

fn validate_binding(
    binding: &ApplicationCheckpointBinding,
) -> Result<(), ApplicationCheckpointError> {
    let operation_valid =
        binding.sequence != 0 && binding.operation_id != [0; 32] && binding.unit_digest != [0; 32];
    let installation_valid = binding.authority.issuer.install_id != [0; 32]
        && binding.authority.receiver.install_id != [0; 32]
        && binding.authority.receiver.component_install_id != [0; 32]
        && binding.authority.policy_digest != [0; 32];
    let component = binding.authority.receiver.component();
    let authority_valid = component.authority == binding.authority.issuer.registry
        && component.fleet_subnet_root == binding.authority.issuer.root;
    if !operation_valid || !installation_valid || !authority_valid {
        return Err(ApplicationCheckpointError::InvalidIdentity);
    }
    Ok(())
}

fn validate_restore_binding(
    captured: &ApplicationCheckpointBinding,
    binding: &ApplicationCheckpointBinding,
) -> Result<(), ApplicationCheckpointError> {
    validate_binding(captured)?;
    validate_binding(binding)?;
    let same_unit =
        captured.authority == binding.authority && captured.unit_digest == binding.unit_digest;
    if !same_unit || binding.sequence <= captured.sequence {
        return Err(ApplicationCheckpointError::InvalidSource);
    }
    Ok(())
}

fn require_current_authority(
    previous: &ApplicationCheckpointRecord,
    binding: &ApplicationCheckpointBinding,
) -> Result<(), ApplicationCheckpointError> {
    if previous.binding.authority != binding.authority {
        return Err(ApplicationCheckpointError::IdentityConflict);
    }
    if previous.binding.sequence > binding.sequence {
        return Err(ApplicationCheckpointError::StaleSequence);
    }
    Ok(())
}

fn require_exact(
    previous: &ApplicationCheckpointRecord,
    binding: &ApplicationCheckpointBinding,
) -> Result<(), ApplicationCheckpointError> {
    if &previous.binding != binding {
        return Err(ApplicationCheckpointError::IdentityConflict);
    }
    Ok(())
}
