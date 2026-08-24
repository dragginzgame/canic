pub mod format {
    pub use crate::format::{byte_size, cycles_tc, truncate};
}

pub mod icp_refill {
    pub use crate::domain::icp_refill::icp_refill_outcome_is_resumable;
}

/// Canonical Fleet admission policy compilation shared with protected host planning.
pub mod fleet_admission_policy {
    pub use crate::domain::policy::pure::fleet_admission::{
        effective_fleet_admission_principals, effective_fleet_admission_template_principals,
    };
    pub use crate::model::{
        fleet_admission_policy::FleetAdmissionPolicyValidationError,
        fleet_admission_projection::FleetAdmissionProjectionValidationError,
    };
    pub use crate::ops::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
        compile_installed_fleet_admission_policy, expected_fleet_admission_target_receipt,
        fleet_admission_participant_catalog_digest, fleet_admission_projection_digest,
        fleet_admission_root_activate_request_digest, fleet_admission_root_open_request_digest,
        fleet_admission_root_participant_catalog_digest, fleet_admission_root_prepare_request,
        fleet_admission_root_prepare_request_digest, fleet_admission_root_receipt_digest,
        fleet_admission_root_receipt_digest_from_binding, fleet_admission_target_for_binding,
        fleet_admission_template_projection_digest, materialize_fleet_admission_projection,
        validate_fleet_admission_policy_template, validate_installed_fleet_admission_policy,
        validate_installed_fleet_admission_projection,
    };
    pub use crate::workflow::fleet_admission_projection::compile_fleet_admission_projection;
}

/// Coordinator-owned Fleet-admission mutation and replay authority shared with the control plane.
pub mod fleet_admission_authority {
    pub use crate::domain::policy::pure::fleet_admission::{
        FleetAdmissionAuthorityPolicyError, FleetAdmissionMembershipMutation,
        FleetAdmissionMutationDecision, FleetAdmissionMutationPolicyError,
        mutate_fleet_admission_membership, plan_fleet_admission_mutation,
    };
    pub use crate::model::fleet_admission_authority::{
        FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION, FleetAdmissionAuthorityState,
        FleetAdmissionCoordinatorRootPhaseModel, FleetAdmissionCoordinatorRootProgressModel,
        FleetAdmissionCoordinatorTransitionPhaseModel, FleetAdmissionMutationActionModel,
        FleetAdmissionMutationOperationInput, FleetAdmissionMutationOutcomeModel,
        FleetAdmissionMutationRequestModel, FleetAdmissionMutationResponseModel,
        FleetAdmissionRetainedResultModel, FleetAdmissionRootCatalogAuthorityModel,
        FleetAdmissionTransitionModel, MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES,
        MAX_FLEET_ADMISSION_PUBLICATIONS, MAX_FLEET_ADMISSION_STATUS_PAGE,
    };
    pub use crate::ops::fleet_admission_policy::{
        fleet_admission_mutation_operation_id, fleet_admission_mutation_request_digest,
    };
}

/// Root-owned admission distribution state and pure ordered transitions.
pub mod fleet_admission_root {
    pub use crate::domain::policy::pure::fleet_admission_root::{
        FleetAdmissionRootPrepareDecision, FleetAdmissionRootTransitionError,
        activate_fleet_admission_root, complete_fleet_admission_root, fence_fleet_admission_root,
        open_fleet_admission_root, prepare_fleet_admission_root,
        record_fleet_admission_root_participant, release_fleet_admission_root,
        validate_fleet_admission_root_state,
    };
    pub use crate::model::fleet_admission_root::{
        FLEET_ADMISSION_ROOT_SCHEMA_VERSION, FleetAdmissionRootParticipantModel,
        FleetAdmissionRootParticipantPhaseModel, FleetAdmissionRootPhaseModel,
        FleetAdmissionRootPrepareRequestModel, FleetAdmissionRootReleasedReservationModel,
        FleetAdmissionRootRetainedResultModel, FleetAdmissionRootState,
        FleetAdmissionRootTransitionModel, MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS,
        MAX_FLEET_ADMISSION_ROOT_RECORD_BYTES, MAX_FLEET_ADMISSION_ROOT_STATUS_PAGE,
    };
}

/// Canonical immutable root-funding policy identities shared with host planning.
pub mod fleet_funding_policy {
    pub use crate::model::fleet_funding_policy::{
        FleetFundingPolicyRotationValidationError, FleetFundingPolicyValidationError,
        validate_coordinator_root_funding_policy, validate_fleet_root_funding_admission,
        validate_fleet_root_funding_capacity, validate_fleet_subnet_root_funding_authority,
    };
    pub use crate::ops::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
        fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
        fleet_funding_policy_rotation_successor_policy_set_hash, fleet_root_funding_operation_id,
        fleet_subnet_root_funding_policy_hash, validate_fleet_funding_policy_rotation_plan,
    };
}

/// Return whether a name uses canonical lowercase ASCII snake_case.
#[must_use]
pub const fn is_ascii_snake_case(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }

    let mut index = 1;
    let mut previous_was_underscore = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_was_underscore = false;
        } else if byte == b'_' && !previous_was_underscore {
            previous_was_underscore = true;
        } else {
            return false;
        }
        index += 1;
    }

    !previous_was_underscore
}
