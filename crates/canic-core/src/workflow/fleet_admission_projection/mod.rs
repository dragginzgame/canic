//! Module: workflow::fleet_admission_projection
//!
//! Responsibility: coordinate fresh projection retention, activation, restore and bounded status.
//! Does not own: endpoint authentication, stable encoding, policy compilation, or convergence.
//! Boundary: lifecycle and API facades call this after acquiring exact managed authority.

use crate::domain::policy::pure::fleet_admission_projection::{
    FleetAdmissionTargetTransitionError, transition_fleet_admission_projection,
};
use crate::{
    InternalError,
    domain::policy::pure::{
        fleet_admission::effective_fleet_admission_principals,
        fleet_admission_projection::open_fresh_fleet_admission_projection,
    },
    dto::{
        fleet_activation::FleetActivationPhase,
        fleet_admission::{
            FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
            FleetAdmissionPrepareTargetRequest, FleetAdmissionPreparedProjectionStatus,
            FleetAdmissionProjectionPhase, FleetAdmissionProjectionStatusResponse,
            FleetAdmissionTargetReceipt,
        },
        page::{Page, PageRequest},
    },
    ids::{
        FleetAdmissionPolicy, FleetAdmissionProjection, MAX_FLEET_ADMISSION_PROJECTION_PAGE,
        ManagedCanisterBinding,
    },
    model::fleet_admission_projection::{
        FleetAdmissionProjectionPhaseModel, FleetAdmissionProjectionState,
        FleetAdmissionProjectionValidationError,
    },
    ops::{
        fleet_admission_policy::{
            fleet_admission_activate_target_request, fleet_admission_open_target_request,
            fleet_admission_prepare_target_request, fleet_admission_target_for_binding,
            fleet_admission_target_receipt, materialize_fleet_admission_projection,
        },
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{
            fleet_activation::FleetActivationOps,
            fleet_admission_projection::FleetAdmissionProjectionOps,
        },
    },
};

/// Apply the sole effective-membership policy, then materialize one exact target projection.
pub fn compile_fleet_admission_projection(
    policy: &FleetAdmissionPolicy,
    target: ManagedCanisterBinding,
) -> Result<FleetAdmissionProjection, FleetAdmissionProjectionValidationError> {
    let selector_target = fleet_admission_target_for_binding(&target);
    let principals = effective_fleet_admission_principals(policy, &selector_target);
    materialize_fleet_admission_projection(policy, target, principals)
}

/// Orchestrator for the one target-local Fleet admission projection.
pub struct FleetAdmissionProjectionWorkflow;

impl FleetAdmissionProjectionWorkflow {
    /// Retain one exact generation-one projection fenced before application activation.
    pub(crate) fn initialize(projection: FleetAdmissionProjection) -> Result<(), InternalError> {
        let expected = projection.target.clone();
        validate_target_is_self(&expected)?;
        FleetAdmissionProjectionOps::initialize(projection, &expected)
    }

    /// Validate same-release restored state without repair or implicit opening.
    pub(crate) fn restore() -> Result<(), InternalError> {
        let expected = exact_managed_target()?;
        FleetAdmissionProjectionOps::validated(&expected).map(|_state| ())
    }

    /// Open a fresh projection only after the managed Component is durably active.
    pub(crate) fn open_fresh() -> Result<bool, InternalError> {
        let activation = FleetActivationOps::status(false)
            .map_err(crate::ops::storage::StorageOpsError::from)?;
        if activation.phase != FleetActivationPhase::Active {
            return Err(InternalError::conflict());
        }
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        let decision = open_fresh_fleet_admission_projection(&state)
            .map_err(|_error| InternalError::invariant())?;
        if decision.transitioned {
            FleetAdmissionProjectionOps::replace(decision.state)?;
        }
        Ok(decision.transitioned)
    }

    /// Read membership synchronously without mutation or remote lookup.
    pub(crate) fn contains(principal: candid::Principal) -> Result<bool, InternalError> {
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        if state.phase != FleetAdmissionProjectionPhaseModel::Open {
            return Ok(false);
        }
        Ok(state.active.principals.binary_search(&principal).is_ok())
    }

    /// Atomically retain the exact successor and fence protected ingress.
    pub(crate) fn prepare(
        request: FleetAdmissionPrepareTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, InternalError> {
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        let request = fleet_admission_prepare_target_request(request, &expected)
            .map_err(|_error| InternalError::invalid_input())?;
        transition(state, request)
    }

    /// Replace the active projection with the exact prepared successor while fenced.
    pub(crate) fn activate(
        request: FleetAdmissionActivateTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, InternalError> {
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        let request = fleet_admission_activate_target_request(&state, request)
            .map_err(|_error| InternalError::conflict())?;
        transition(state, request)
    }

    /// Open protected ingress on the exact active successor.
    pub(crate) fn open(
        request: FleetAdmissionOpenTargetRequest,
    ) -> Result<FleetAdmissionTargetReceipt, InternalError> {
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        let request = fleet_admission_open_target_request(&state, request)
            .map_err(|_error| InternalError::conflict())?;
        transition(state, request)
    }

    /// Return one bounded protected local status page.
    pub(crate) fn status(
        request: PageRequest,
    ) -> Result<FleetAdmissionProjectionStatusResponse, InternalError> {
        let expected = exact_managed_target()?;
        let state = FleetAdmissionProjectionOps::validated(&expected)?;
        status_from_state(state, request)
    }
}

fn transition(
    state: FleetAdmissionProjectionState,
    request: crate::model::fleet_admission_projection::FleetAdmissionTargetTransitionRequestModel,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    let projection = request.successor.clone();
    let decision =
        transition_fleet_admission_projection(&state, request).map_err(map_transition_error)?;
    if !decision.replayed {
        FleetAdmissionProjectionOps::replace(decision.state)?;
    }
    Ok(fleet_admission_target_receipt(
        &projection,
        &decision.receipt,
    ))
}

const fn map_transition_error(error: FleetAdmissionTargetTransitionError) -> InternalError {
    match error {
        FleetAdmissionTargetTransitionError::EmptyOperationId => InternalError::invalid_input(),
        FleetAdmissionTargetTransitionError::OperationConflict
        | FleetAdmissionTargetTransitionError::PhaseConflict
        | FleetAdmissionTargetTransitionError::SuccessorConflict => InternalError::conflict(),
        FleetAdmissionTargetTransitionError::InvalidState => InternalError::invariant(),
    }
}

fn status_from_state(
    state: FleetAdmissionProjectionState,
    request: PageRequest,
) -> Result<FleetAdmissionProjectionStatusResponse, InternalError> {
    let total =
        u64::try_from(state.active.principals.len()).map_err(|_| InternalError::invariant())?;
    let limit = request.limit.min(MAX_FLEET_ADMISSION_PROJECTION_PAGE);
    let entries = usize::try_from(request.offset)
        .ok()
        .filter(|offset| *offset < state.active.principals.len())
        .map_or_else(Vec::new, |offset| {
            let take = usize::try_from(limit).expect("projection page limit fits usize");
            state
                .active
                .principals
                .iter()
                .skip(offset)
                .take(take)
                .copied()
                .collect()
        });
    Ok(FleetAdmissionProjectionStatusResponse {
        authority: state.active.authority,
        target: state.active.target,
        generation: state.active.generation,
        policy_digest: state.active.policy_digest,
        projection_digest: state.active.projection_digest,
        phase: match state.phase {
            FleetAdmissionProjectionPhaseModel::Fenced => FleetAdmissionProjectionPhase::Fenced,
            FleetAdmissionProjectionPhaseModel::Open => FleetAdmissionProjectionPhase::Open,
        },
        prepared: state
            .prepared
            .map(|projection| FleetAdmissionPreparedProjectionStatus {
                generation: projection.generation,
                policy_digest: projection.policy_digest,
                projection_digest: projection.projection_digest,
            }),
        principals: Page { entries, total },
        maximum_page_size: u16::try_from(MAX_FLEET_ADMISSION_PROJECTION_PAGE)
            .expect("projection page limit fits u16"),
    })
}

fn exact_managed_target() -> Result<ManagedCanisterBinding, InternalError> {
    let target = EnvOps::managed_binding()?;
    validate_target_is_self(&target)?;
    Ok(target)
}

fn validate_target_is_self(target: &ManagedCanisterBinding) -> Result<(), InternalError> {
    let target_canister = match target {
        ManagedCanisterBinding::Component(binding) => binding.canister_id,
        ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
    };
    if target_canister != IcOps::canister_self() {
        return Err(InternalError::invariant());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_status_pages_are_canonical_clamped_and_bounded() {
        let mut projection = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        projection.principals = (0..=MAX_FLEET_ADMISSION_PROJECTION_PAGE)
            .map(|index| candid::Principal::from_slice(&index.to_be_bytes()))
            .collect();
        let state = FleetAdmissionProjectionState {
            schema_version: 1,
            active: projection,
            prepared: None,
            phase: FleetAdmissionProjectionPhaseModel::Fenced,
            last_receipt: None,
        };

        let first = status_from_state(
            state.clone(),
            PageRequest {
                offset: 0,
                limit: u64::MAX,
            },
        )
        .expect("bounded first page");
        assert_eq!(
            first.principals.total,
            MAX_FLEET_ADMISSION_PROJECTION_PAGE + 1
        );
        assert_eq!(first.principals.entries.len(), 128);

        let empty = status_from_state(
            state,
            PageRequest {
                offset: u64::MAX,
                limit: 1,
            },
        )
        .expect("empty out-of-range page");
        assert!(empty.principals.entries.is_empty());
        assert_eq!(
            first.maximum_page_size,
            u16::try_from(MAX_FLEET_ADMISSION_PROJECTION_PAGE).expect("bounded")
        );
        assert!(first.principals.total <= crate::ids::MAX_FLEET_ADMISSION_PRINCIPALS as u64);
    }
}
