//! Module: model::fleet_admission_projection
//!
//! Responsibility: own target-local Fleet admission projection and phase invariants.
//! Does not own: hashing, stable access, caller acquisition, or distributed convergence.
//! Boundary: ops supplies exact bindings and independently computed digest evidence.

use crate::ids::{
    FLEET_ADMISSION_SCHEMA_VERSION, FleetAdmissionProjection, MAX_FLEET_ADMISSION_PRINCIPALS,
};
use thiserror::Error as ThisError;

/// Current local projection record schema.
pub const FLEET_ADMISSION_PROJECTION_STATE_SCHEMA_VERSION: u16 = 1;

/// Whether protected ingress is fenced or serving the active projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionProjectionPhaseModel {
    Fenced,
    Open,
}

/// Closed target-local transition phase retained with one exact receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionTargetTransitionPhaseModel {
    Prepare,
    Activate,
    Open,
}

impl FleetAdmissionTargetTransitionPhaseModel {
    /// Return the canonical request/receipt hash discriminator.
    #[must_use]
    pub const fn hash_byte(self) -> u8 {
        match self {
            Self::Prepare => 0,
            Self::Activate => 1,
            Self::Open => 2,
        }
    }
}

/// Complete layer-neutral target transition request after DTO conversion and hashing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionTargetTransitionRequestModel {
    pub operation_id: [u8; 32],
    pub phase: FleetAdmissionTargetTransitionPhaseModel,
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: FleetAdmissionProjection,
    pub request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// One retained participant transition receipt for same-release exact retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionProjectionReceiptModel {
    pub operation_id: [u8; 32],
    pub phase: FleetAdmissionTargetTransitionPhaseModel,
    pub request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Sole target-local Fleet admission enforcement authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionProjectionState {
    pub schema_version: u16,
    pub active: FleetAdmissionProjection,
    pub prepared: Option<FleetAdmissionProjection>,
    pub phase: FleetAdmissionProjectionPhaseModel,
    pub last_receipt: Option<FleetAdmissionProjectionReceiptModel>,
}

/// DTO-free facts used to validate one complete projection.
pub struct FleetAdmissionProjectionValidationInput<'a> {
    pub projection: &'a FleetAdmissionProjection,
    pub expected_target: &'a crate::ids::ManagedCanisterBinding,
    pub digest_matches: bool,
}

/// Exact local projection invariant rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionProjectionValidationError {
    #[error("Fleet admission projection schema is unsupported")]
    UnsupportedSchema,
    #[error("Fleet admission projection generation must be positive")]
    GenerationZero,
    #[error("Fleet admission projection target does not match installed authority")]
    TargetMismatch,
    #[error("Fleet admission projection Coordinator authority does not match its target")]
    AuthorityMismatch,
    #[error("Fleet admission projection Principal count exceeds 256")]
    PrincipalCountExceeded,
    #[error("Fleet admission projection Principals are not canonical")]
    PrincipalsNonCanonical,
    #[error("Fleet admission projection contains the anonymous Principal")]
    AnonymousPrincipal,
    #[error("Fleet admission projection policy digest is absent")]
    PolicyDigestMissing,
    #[error("Fleet admission projection digest is invalid")]
    ProjectionDigestMismatch,
    #[error("Fleet admission prepared projection is not the exact successor")]
    PreparedProjectionInvalid,
    #[error("Fleet admission open projection retains transition-only state")]
    OpenStateInvalid,
    #[error("Fleet admission retained receipt is invalid")]
    RetainedReceiptInvalid,
}

/// Validate one target-bound projection without reading ambient state.
pub fn validate_fleet_admission_projection(
    input: &FleetAdmissionProjectionValidationInput<'_>,
) -> Result<(), FleetAdmissionProjectionValidationError> {
    let projection = input.projection;
    if projection.schema_version != FLEET_ADMISSION_SCHEMA_VERSION {
        return Err(FleetAdmissionProjectionValidationError::UnsupportedSchema);
    }
    if projection.generation == 0 {
        return Err(FleetAdmissionProjectionValidationError::GenerationZero);
    }
    if &projection.target != input.expected_target {
        return Err(FleetAdmissionProjectionValidationError::TargetMismatch);
    }
    if projection.authority != *projection_target_authority(input.expected_target) {
        return Err(FleetAdmissionProjectionValidationError::AuthorityMismatch);
    }
    if projection.principals.len() > MAX_FLEET_ADMISSION_PRINCIPALS {
        return Err(FleetAdmissionProjectionValidationError::PrincipalCountExceeded);
    }
    if projection
        .principals
        .iter()
        .any(|principal| principal == &candid::Principal::anonymous())
    {
        return Err(FleetAdmissionProjectionValidationError::AnonymousPrincipal);
    }
    if projection
        .principals
        .windows(2)
        .any(|items| items[0] >= items[1])
    {
        return Err(FleetAdmissionProjectionValidationError::PrincipalsNonCanonical);
    }
    if projection.policy_digest == [0; 32] {
        return Err(FleetAdmissionProjectionValidationError::PolicyDigestMissing);
    }
    if !input.digest_matches {
        return Err(FleetAdmissionProjectionValidationError::ProjectionDigestMismatch);
    }
    Ok(())
}

/// Validate state relationships after ops validates both complete digests.
pub fn validate_fleet_admission_projection_state(
    state: &FleetAdmissionProjectionState,
) -> Result<(), FleetAdmissionProjectionValidationError> {
    if state.schema_version != FLEET_ADMISSION_PROJECTION_STATE_SCHEMA_VERSION {
        return Err(FleetAdmissionProjectionValidationError::UnsupportedSchema);
    }
    if let Some(prepared) = &state.prepared {
        let expected_generation = state
            .active
            .generation
            .checked_add(1)
            .ok_or(FleetAdmissionProjectionValidationError::PreparedProjectionInvalid)?;
        let same_target =
            prepared.authority == state.active.authority && prepared.target == state.active.target;
        if prepared.generation != expected_generation || !same_target {
            return Err(FleetAdmissionProjectionValidationError::PreparedProjectionInvalid);
        }
    }
    if state.phase == FleetAdmissionProjectionPhaseModel::Open && state.prepared.is_some() {
        return Err(FleetAdmissionProjectionValidationError::OpenStateInvalid);
    }
    if state.last_receipt.as_ref().is_some_and(|receipt| {
        receipt.operation_id == [0; 32]
            || receipt.request_hash == [0; 32]
            || receipt.receipt_hash == [0; 32]
    }) {
        return Err(FleetAdmissionProjectionValidationError::RetainedReceiptInvalid);
    }
    match (&state.prepared, state.phase, &state.last_receipt) {
        (Some(_), FleetAdmissionProjectionPhaseModel::Fenced, Some(receipt))
            if receipt.phase == FleetAdmissionTargetTransitionPhaseModel::Prepare => {}
        (None, FleetAdmissionProjectionPhaseModel::Fenced, Some(receipt))
            if receipt.phase == FleetAdmissionTargetTransitionPhaseModel::Activate => {}
        (None, FleetAdmissionProjectionPhaseModel::Open, Some(receipt))
            if receipt.phase == FleetAdmissionTargetTransitionPhaseModel::Open => {}
        (
            None,
            FleetAdmissionProjectionPhaseModel::Fenced | FleetAdmissionProjectionPhaseModel::Open,
            None,
        ) => {}
        _ => return Err(FleetAdmissionProjectionValidationError::RetainedReceiptInvalid),
    }
    Ok(())
}

const fn projection_target_authority(
    target: &crate::ids::ManagedCanisterBinding,
) -> &crate::ids::FleetCoordinatorBinding {
    match target {
        crate::ids::ManagedCanisterBinding::Component(binding) => &binding.authority.binding,
        crate::ids::ManagedCanisterBinding::ComponentChild(binding) => {
            &binding.component.authority.binding
        }
    }
}
