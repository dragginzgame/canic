//! Module: ops::storage::fleet_admission_projection
//!
//! Responsibility: access and convert complete target-local admission state atomically.
//! Does not own: phase decisions, lifecycle order, endpoint authorization, or distribution.
//! Boundary: workflow receives model state and commits only policy-produced replacements.

use crate::{
    InternalError,
    ids::ManagedCanisterBinding,
    model::fleet_admission_projection::{
        FleetAdmissionProjectionPhaseModel, FleetAdmissionProjectionReceiptModel,
        FleetAdmissionProjectionState, validate_fleet_admission_projection_state,
    },
    ops::fleet_admission_policy::validate_installed_fleet_admission_projection,
    storage::stable::fleet_admission_projection::{
        FleetAdmissionProjectionPhaseRecord, FleetAdmissionProjectionReceiptRecord,
        FleetAdmissionProjectionRecord, FleetAdmissionProjectionStore,
    },
};

/// Deterministic storage facade for the reused memory ID 61.
pub struct FleetAdmissionProjectionOps;

impl FleetAdmissionProjectionOps {
    /// Load the optional record without inventing authority.
    pub(crate) fn load() -> Option<FleetAdmissionProjectionState> {
        FleetAdmissionProjectionStore::get().map(record_to_model)
    }

    /// Commit the fresh fenced projection exactly once.
    pub(crate) fn initialize(
        projection: crate::ids::FleetAdmissionProjection,
        expected_target: &ManagedCanisterBinding,
    ) -> Result<(), InternalError> {
        validate_installed_fleet_admission_projection(&projection, expected_target)
            .map_err(|_error| InternalError::invariant())?;
        let state = FleetAdmissionProjectionState {
            schema_version: 1,
            active: projection,
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Fenced,
            last_receipt: None,
        };
        validate_fleet_admission_projection_state(&state)
            .map_err(|_error| InternalError::invariant())?;
        if FleetAdmissionProjectionStore::initialize(model_to_record(state)) {
            Ok(())
        } else {
            Err(InternalError::conflict())
        }
    }

    /// Validate the complete restored record against the exact stable binding.
    pub(crate) fn validated(
        expected_target: &ManagedCanisterBinding,
    ) -> Result<FleetAdmissionProjectionState, InternalError> {
        let state = Self::load().ok_or_else(InternalError::unavailable)?;
        validate_state(&state, expected_target)?;
        Ok(state)
    }

    /// Atomically replace an existing complete record.
    pub(crate) fn replace(state: FleetAdmissionProjectionState) -> Result<(), InternalError> {
        if FleetAdmissionProjectionStore::replace(model_to_record(state)) {
            Ok(())
        } else {
            Err(InternalError::unavailable())
        }
    }
}

fn validate_state(
    state: &FleetAdmissionProjectionState,
    expected_target: &ManagedCanisterBinding,
) -> Result<(), InternalError> {
    validate_installed_fleet_admission_projection(&state.active, expected_target)
        .map_err(|_error| InternalError::invariant())?;
    if let Some(prepared) = &state.prepared {
        validate_installed_fleet_admission_projection(prepared, expected_target)
            .map_err(|_error| InternalError::invariant())?;
    }
    validate_fleet_admission_projection_state(state).map_err(|_error| InternalError::invariant())
}

fn record_to_model(record: FleetAdmissionProjectionRecord) -> FleetAdmissionProjectionState {
    FleetAdmissionProjectionState {
        schema_version: record.schema_version,
        active: record.active,
        prepared: record.prepared,
        phase: match record.phase {
            FleetAdmissionProjectionPhaseRecord::Fenced => {
                FleetAdmissionProjectionPhaseModel::Fenced
            }
            FleetAdmissionProjectionPhaseRecord::Open => FleetAdmissionProjectionPhaseModel::Open,
        },
        last_receipt: record
            .last_receipt
            .map(|receipt| FleetAdmissionProjectionReceiptModel {
                operation_id: receipt.operation_id,
                phase: receipt.phase.into(),
                request_hash: receipt.request_hash,
                receipt_hash: receipt.receipt_hash,
            }),
    }
}

fn model_to_record(state: FleetAdmissionProjectionState) -> FleetAdmissionProjectionRecord {
    FleetAdmissionProjectionRecord {
        schema_version: state.schema_version,
        active: state.active,
        prepared: state.prepared,
        phase: match state.phase {
            FleetAdmissionProjectionPhaseModel::Fenced => {
                FleetAdmissionProjectionPhaseRecord::Fenced
            }
            FleetAdmissionProjectionPhaseModel::Open => FleetAdmissionProjectionPhaseRecord::Open,
        },
        last_receipt: state
            .last_receipt
            .map(|receipt| FleetAdmissionProjectionReceiptRecord {
                operation_id: receipt.operation_id,
                phase: receipt.phase.into(),
                request_hash: receipt.request_hash,
                receipt_hash: receipt.receipt_hash,
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_conversion_preserves_complete_projection_authority() {
        let state = FleetAdmissionProjectionState {
            schema_version: 1,
            active: crate::test::support::fleet_admission_projection(
                crate::test::support::managed_component_binding(),
            ),
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Fenced,
            last_receipt: None,
        };

        assert_eq!(record_to_model(model_to_record(state.clone())), state);
    }
}
