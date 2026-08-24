//! Module: ops::fleet_admission_policy
//!
//! Responsibility: compile and hash canonical Fleet admission templates and installed policies.
//! Does not own: transport parsing, selector decisions, persistence, mutation, or orchestration.
//! Boundary: host and canister workflows call this after acquiring exact protected identities.

#[cfg(test)]
mod tests;

use crate::{
    dto::fleet_admission::{
        FleetAdmissionActivateRootRequest, FleetAdmissionActivateTargetRequest,
        FleetAdmissionOpenRootRequest, FleetAdmissionOpenTargetRequest,
        FleetAdmissionPrepareRootRequest, FleetAdmissionPrepareRootStage,
        FleetAdmissionPrepareTargetRequest, FleetAdmissionRootTransitionPhase,
        FleetAdmissionTargetReceipt, FleetAdmissionTargetTransitionPhase,
    },
    dto::fleet_registry::FleetRegistryVersion,
    ids::{
        FLEET_ADMISSION_INITIAL_GENERATION, FLEET_ADMISSION_SCHEMA_VERSION, FleetAdmissionPolicy,
        FleetAdmissionPolicyTemplate, FleetAdmissionProjection, FleetAdmissionRule,
        FleetAdmissionSelector, FleetAdmissionTarget, FleetBinding, FleetCoordinatorBinding,
        ManagedCanisterBinding, SubnetId,
    },
    model::fleet_admission_authority::{
        FleetAdmissionMutationOperationInput, FleetAdmissionMutationRequestModel,
        FleetAdmissionRootCatalogAuthorityModel,
    },
    model::fleet_admission_policy::{
        FleetAdmissionPolicyValidationError, FleetAdmissionPolicyValidationInput,
        validate_fleet_admission_policy, validate_initial_fleet_admission_generation,
    },
    model::fleet_admission_projection::{
        FleetAdmissionProjectionReceiptModel, FleetAdmissionProjectionState,
        FleetAdmissionProjectionValidationError, FleetAdmissionProjectionValidationInput,
        FleetAdmissionTargetTransitionPhaseModel, FleetAdmissionTargetTransitionRequestModel,
        validate_fleet_admission_projection,
    },
    model::fleet_admission_root::FleetAdmissionRootPrepareRequestModel,
};
use candid::Principal;
use sha2::{Digest, Sha256};

const TEMPLATE_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-template/v1";
const POLICY_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-policy/v1";
const TEMPLATE_PROJECTION_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-template-projection/v1";
const PROJECTION_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-projection/v1";
const MUTATION_REQUEST_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-mutation-request/v1";
const MUTATION_OPERATION_ID_DOMAIN: &[u8] = b"canic/fleet-admission-mutation-operation/v1";
const TARGET_TRANSITION_REQUEST_DIGEST_DOMAIN: &[u8] =
    b"canic/fleet-admission-target-transition-request/v1";
const TARGET_TRANSITION_RECEIPT_DIGEST_DOMAIN: &[u8] =
    b"canic/fleet-admission-target-transition-receipt/v1";
const ROOT_PREPARE_REQUEST_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-root-prepare-request/v1";
const ROOT_ACTIVATE_REQUEST_DIGEST_DOMAIN: &[u8] =
    b"canic/fleet-admission-root-activate-request/v1";
const ROOT_OPEN_REQUEST_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-root-open-request/v1";
const ROOT_PARTICIPANT_CATALOG_DIGEST_DOMAIN: &[u8] =
    b"canic/fleet-admission-root-participant-catalog/v1";
const PARTICIPANT_CATALOG_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-participant-catalog/v1";
const ROOT_RECEIPT_DIGEST_DOMAIN: &[u8] = b"canic/fleet-admission-root-receipt/v1";

/// Compile one already-canonical protected generation-one policy template.
pub fn compile_fleet_admission_policy_template(
    fleet_principals: Vec<Principal>,
    rules: Vec<FleetAdmissionRule>,
) -> Result<FleetAdmissionPolicyTemplate, FleetAdmissionPolicyValidationError> {
    let template_digest = template_digest(&fleet_principals, &rules);
    let template = FleetAdmissionPolicyTemplate {
        schema_version: FLEET_ADMISSION_SCHEMA_VERSION,
        fleet_principals,
        rules,
        template_digest,
    };
    validate_fleet_admission_policy_template(&template)?;
    Ok(template)
}

/// Validate the complete canonical template and retained digest.
pub fn validate_fleet_admission_policy_template(
    template: &FleetAdmissionPolicyTemplate,
) -> Result<(), FleetAdmissionPolicyValidationError> {
    validate_fleet_admission_policy(&FleetAdmissionPolicyValidationInput {
        schema_version: template.schema_version,
        generation: None,
        fleet_principals: &template.fleet_principals,
        rules: &template.rules,
        digest_matches: template.template_digest
            == template_digest(&template.fleet_principals, &template.rules),
    })
}

/// Bind one validated template to the exact newly allocated Fleet identity.
pub fn bind_initial_fleet_admission_policy(
    fleet: FleetBinding,
    template: &FleetAdmissionPolicyTemplate,
) -> Result<FleetAdmissionPolicy, FleetAdmissionPolicyValidationError> {
    validate_fleet_admission_policy_template(template)?;
    validate_initial_fleet_admission_generation(FLEET_ADMISSION_INITIAL_GENERATION)?;
    compile_installed_fleet_admission_policy(
        fleet,
        FLEET_ADMISSION_INITIAL_GENERATION,
        template.fleet_principals.clone(),
        template.rules.clone(),
    )
}

/// Compile one complete Fleet-bound policy at an explicit positive generation.
pub fn compile_installed_fleet_admission_policy(
    fleet: FleetBinding,
    generation: u64,
    fleet_principals: Vec<Principal>,
    rules: Vec<FleetAdmissionRule>,
) -> Result<FleetAdmissionPolicy, FleetAdmissionPolicyValidationError> {
    let policy_digest = policy_digest(&fleet, generation, &fleet_principals, &rules);
    let policy = FleetAdmissionPolicy {
        schema_version: FLEET_ADMISSION_SCHEMA_VERSION,
        fleet,
        generation,
        fleet_principals,
        rules,
        policy_digest,
    };
    validate_installed_fleet_admission_policy(&policy)?;
    Ok(policy)
}

/// Validate one complete installed policy and its exact Fleet-bound digest.
pub fn validate_installed_fleet_admission_policy(
    policy: &FleetAdmissionPolicy,
) -> Result<(), FleetAdmissionPolicyValidationError> {
    validate_fleet_admission_policy(&FleetAdmissionPolicyValidationInput {
        schema_version: policy.schema_version,
        generation: Some(policy.generation),
        fleet_principals: &policy.fleet_principals,
        rules: &policy.rules,
        digest_matches: policy.policy_digest
            == policy_digest(
                &policy.fleet,
                policy.generation,
                &policy.fleet_principals,
                &policy.rules,
            ),
    })
}

/// Compile one complete projection for an exact managed target.
pub fn materialize_fleet_admission_projection(
    policy: &FleetAdmissionPolicy,
    target: ManagedCanisterBinding,
    principals: Vec<Principal>,
) -> Result<FleetAdmissionProjection, FleetAdmissionProjectionValidationError> {
    validate_installed_fleet_admission_policy(policy)
        .map_err(|_error| FleetAdmissionProjectionValidationError::AuthorityMismatch)?;
    let authority = projection_target_authority(&target).clone();
    if authority.fleet != policy.fleet {
        return Err(FleetAdmissionProjectionValidationError::AuthorityMismatch);
    }
    let projection_digest = fleet_admission_projection_digest(
        &authority,
        &target,
        policy.generation,
        policy.policy_digest,
        &principals,
    );
    let projection = FleetAdmissionProjection {
        schema_version: FLEET_ADMISSION_SCHEMA_VERSION,
        authority,
        target: target.clone(),
        generation: policy.generation,
        policy_digest: policy.policy_digest,
        projection_digest,
        principals,
    };
    validate_installed_fleet_admission_projection(&projection, &target)?;
    Ok(projection)
}

/// Validate one complete projection against its exact installed target.
pub fn validate_installed_fleet_admission_projection(
    projection: &FleetAdmissionProjection,
    expected_target: &ManagedCanisterBinding,
) -> Result<(), FleetAdmissionProjectionValidationError> {
    validate_fleet_admission_projection(&FleetAdmissionProjectionValidationInput {
        projection,
        expected_target,
        digest_matches: projection.projection_digest
            == fleet_admission_projection_digest(
                &projection.authority,
                &projection.target,
                projection.generation,
                projection.policy_digest,
                &projection.principals,
            ),
    })
}

/// Derive the selector facts of one exact managed target.
#[must_use]
pub fn fleet_admission_target_for_binding(target: &ManagedCanisterBinding) -> FleetAdmissionTarget {
    let component = match target {
        ManagedCanisterBinding::Component(binding) => binding,
        ManagedCanisterBinding::ComponentChild(binding) => &binding.component,
    };
    FleetAdmissionTarget {
        component_spec: component.component_spec.clone(),
        component_instance: Some(component.component),
        fleet_subnet_root: component.placement_subnet,
    }
}

/// Hash one pre-allocation effective projection under its exact template and target authority.
#[must_use]
pub fn fleet_admission_template_projection_digest(
    template_digest: [u8; 32],
    target: &crate::ids::FleetAdmissionTarget,
    effective_principals: &[Principal],
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(TEMPLATE_PROJECTION_DIGEST_DOMAIN);
    encoder.bytes(&template_digest);
    encoder.string(target.component_spec.as_str());
    match target.component_instance {
        Some(component_instance) => {
            encoder.u8(1);
            encoder.bytes(component_instance.as_bytes());
        }
        None => encoder.u8(0),
    }
    encoder.bytes(target.fleet_subnet_root.as_principal().as_slice());
    encoder.u64(effective_principals.len() as u64);
    for principal in effective_principals {
        encoder.bytes(principal.as_slice());
    }
    encoder.finish()
}

/// Hash every authority-bearing field of one exact installed projection.
#[must_use]
pub fn fleet_admission_projection_digest(
    authority: &crate::ids::FleetCoordinatorBinding,
    target: &ManagedCanisterBinding,
    generation: u64,
    policy_digest: [u8; 32],
    principals: &[Principal],
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(PROJECTION_DIGEST_DOMAIN);
    encode_coordinator_binding(&mut encoder, authority);
    encode_managed_target(&mut encoder, target);
    encoder.u64(generation);
    encoder.bytes(&policy_digest);
    encoder.u64(principals.len() as u64);
    for principal in principals {
        encoder.bytes(principal.as_slice());
    }
    encoder.finish()
}

/// Hash every authority-bearing field of one exact mutation request.
#[must_use]
pub fn fleet_admission_mutation_request_digest(
    request: &FleetAdmissionMutationRequestModel,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(MUTATION_REQUEST_DIGEST_DOMAIN);
    let binding = &request.authority;
    encoder.bytes(binding.fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(binding.fleet.fleet.fleet_id.as_bytes());
    encoder.string(binding.fleet.app.as_str());
    encoder.bytes(binding.coordinator_subnet.as_principal().as_slice());
    encoder.bytes(binding.coordinator.as_slice());
    encoder.u64(request.expected_generation);
    encoder.bytes(&request.expected_policy_digest);
    encoder.u8(request.action.hash_byte());
    encode_selector(&mut encoder, &request.selector);
    encoder.bytes(request.principal.as_slice());
    encoder.bytes(&request.operation_id);
    encoder.bytes(&request.successor_policy_digest);
    encoder.bytes(&request.participant_catalog_digest);
    encoder.u32(request.participant_count);
    encoder.finish()
}

/// Derive one exact operator-plan operation identity from immutable live authority.
#[must_use]
pub fn fleet_admission_mutation_operation_id(
    registry: &FleetRegistryVersion,
    input: &FleetAdmissionMutationOperationInput,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(MUTATION_OPERATION_ID_DOMAIN);
    encode_coordinator_binding(&mut encoder, &registry.authority.binding);
    encoder.u64(registry.authority.epoch);
    encoder.u64(registry.revision);
    encoder.bytes(&registry.content_hash);
    encoder.u64(input.expected_generation);
    encoder.bytes(&input.expected_policy_digest);
    encoder.u8(input.action.hash_byte());
    encode_selector(&mut encoder, &input.selector);
    encoder.bytes(input.principal.as_slice());
    encoder.bytes(&input.successor_policy_digest);
    encoder.bytes(&input.participant_catalog_digest);
    encoder.u32(input.participant_count);
    encoder.finish()
}

/// Convert and bind one target prepare command without reading transport identity.
pub fn fleet_admission_prepare_target_request(
    request: FleetAdmissionPrepareTargetRequest,
    expected_target: &ManagedCanisterBinding,
) -> Result<FleetAdmissionTargetTransitionRequestModel, FleetAdmissionProjectionValidationError> {
    validate_installed_fleet_admission_projection(&request.successor, expected_target)?;
    compile_target_transition_request(
        request.operation_id,
        FleetAdmissionTargetTransitionPhaseModel::Prepare,
        request.expected_generation,
        request.expected_policy_digest,
        request.successor,
    )
}

/// Convert and bind one target activate command to the exact retained successor.
pub fn fleet_admission_activate_target_request(
    state: &FleetAdmissionProjectionState,
    request: FleetAdmissionActivateTargetRequest,
) -> Result<FleetAdmissionTargetTransitionRequestModel, FleetAdmissionProjectionValidationError> {
    let replaying_retained_activation = state.last_receipt.as_ref().is_some_and(|receipt| {
        receipt.operation_id == request.operation_id
            && receipt.phase == FleetAdmissionTargetTransitionPhaseModel::Activate
    });
    let successor = state
        .prepared
        .clone()
        .or_else(|| replaying_retained_activation.then(|| state.active.clone()))
        .ok_or(FleetAdmissionProjectionValidationError::PreparedProjectionInvalid)?;
    let predecessor_matches = if replaying_retained_activation {
        request
            .expected_generation
            .checked_add(1)
            .is_some_and(|generation| generation == state.active.generation)
    } else {
        request.expected_generation == state.active.generation
            && request.expected_policy_digest == state.active.policy_digest
    };
    let request_matches = predecessor_matches
        && request.successor_generation == successor.generation
        && request.successor_policy_digest == successor.policy_digest
        && request.successor_projection_digest == successor.projection_digest;
    if !request_matches {
        return Err(FleetAdmissionProjectionValidationError::PreparedProjectionInvalid);
    }
    compile_target_transition_request(
        request.operation_id,
        FleetAdmissionTargetTransitionPhaseModel::Activate,
        request.expected_generation,
        request.expected_policy_digest,
        successor,
    )
}

/// Convert and bind one target open command to the exact fenced active successor.
pub fn fleet_admission_open_target_request(
    state: &FleetAdmissionProjectionState,
    request: FleetAdmissionOpenTargetRequest,
) -> Result<FleetAdmissionTargetTransitionRequestModel, FleetAdmissionProjectionValidationError> {
    let request_matches = request.generation == state.active.generation
        && request.policy_digest == state.active.policy_digest
        && request.projection_digest == state.active.projection_digest;
    if !request_matches {
        return Err(FleetAdmissionProjectionValidationError::PreparedProjectionInvalid);
    }
    compile_target_transition_request(
        request.operation_id,
        FleetAdmissionTargetTransitionPhaseModel::Open,
        request.generation,
        request.policy_digest,
        state.active.clone(),
    )
}

/// Project one retained model receipt into its exact public target receipt.
#[must_use]
pub fn fleet_admission_target_receipt(
    projection: &FleetAdmissionProjection,
    receipt: &FleetAdmissionProjectionReceiptModel,
) -> FleetAdmissionTargetReceipt {
    FleetAdmissionTargetReceipt {
        operation_id: receipt.operation_id,
        phase: match receipt.phase {
            FleetAdmissionTargetTransitionPhaseModel::Prepare => {
                FleetAdmissionTargetTransitionPhase::Prepare
            }
            FleetAdmissionTargetTransitionPhaseModel::Activate => {
                FleetAdmissionTargetTransitionPhase::Activate
            }
            FleetAdmissionTargetTransitionPhaseModel::Open => {
                FleetAdmissionTargetTransitionPhase::Open
            }
        },
        target: projection.target.clone(),
        generation: projection.generation,
        policy_digest: projection.policy_digest,
        projection_digest: projection.projection_digest,
        receipt_hash: receipt.receipt_hash,
    }
}

/// Bind and hash one complete Coordinator-authored Root prepare request.
pub fn fleet_admission_root_prepare_request(
    request: FleetAdmissionPrepareRootRequest,
    root: crate::ids::FleetSubnetRootBinding,
) -> Result<FleetAdmissionRootPrepareRequestModel, FleetAdmissionProjectionValidationError> {
    validate_installed_fleet_admission_policy(&request.successor)
        .map_err(|_error| FleetAdmissionProjectionValidationError::AuthorityMismatch)?;
    let request_hash = fleet_admission_root_prepare_request_digest(&request, &root);
    Ok(FleetAdmissionRootPrepareRequestModel {
        authority: request.authority,
        root,
        operation_id: request.operation_id,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        successor: request.successor,
        request_hash,
    })
}

/// Hash one exact Root activation command.
#[must_use]
pub fn fleet_admission_root_activate_request_digest(
    request: &FleetAdmissionActivateRootRequest,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(ROOT_ACTIVATE_REQUEST_DIGEST_DOMAIN);
    encode_coordinator_binding(&mut encoder, &request.authority);
    encoder.bytes(&request.operation_id);
    encoder.u64(request.expected_generation);
    encoder.bytes(&request.expected_policy_digest);
    encoder.u64(request.successor_generation);
    encoder.bytes(&request.successor_policy_digest);
    encoder.finish()
}

/// Hash one exact Root open command.
#[must_use]
pub fn fleet_admission_root_open_request_digest(
    request: &FleetAdmissionOpenRootRequest,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(ROOT_OPEN_REQUEST_DIGEST_DOMAIN);
    encode_coordinator_binding(&mut encoder, &request.authority);
    encoder.bytes(&request.operation_id);
    encoder.u64(request.generation);
    encoder.bytes(&request.policy_digest);
    encoder.finish()
}

/// Hash the canonical target/projection snapshot owned by one Root operation.
#[must_use]
pub fn fleet_admission_root_participant_catalog_digest(
    projections: &[FleetAdmissionProjection],
) -> [u8; 32] {
    let mut encoder =
        CanonicalAdmissionEncoder::with_domain(ROOT_PARTICIPANT_CATALOG_DIGEST_DOMAIN);
    encoder.u64(projections.len() as u64);
    for projection in projections {
        encode_managed_target(&mut encoder, &projection.target);
        encoder.bytes(&projection.projection_digest);
    }
    encoder.finish()
}

/// Hash the canonical ordered Root participant-catalog authorities for one Fleet mutation.
#[must_use]
pub fn fleet_admission_participant_catalog_digest(
    catalogs: &[FleetAdmissionRootCatalogAuthorityModel],
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(PARTICIPANT_CATALOG_DIGEST_DOMAIN);
    encode_root_catalogs(&mut encoder, catalogs);
    encoder.finish()
}

/// Hash one exact aggregate Root receipt.
#[must_use]
pub fn fleet_admission_root_receipt_digest(
    operation_id: [u8; 32],
    phase: FleetAdmissionRootTransitionPhase,
    root: &crate::ids::FleetSubnetRootBinding,
    generation: u64,
    policy_digest: [u8; 32],
    participant_catalog_digest: [u8; 32],
    participant_count: u32,
) -> [u8; 32] {
    fleet_admission_root_receipt_digest_from_binding(
        operation_id,
        phase,
        &root.authority.binding,
        root.placement_subnet,
        root.fleet_subnet_root,
        generation,
        policy_digest,
        participant_catalog_digest,
        participant_count,
    )
}

/// Hash one exact aggregate Root receipt from its retained minimal binding.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the frozen receipt identity binds each authority field independently"
)]
pub fn fleet_admission_root_receipt_digest_from_binding(
    operation_id: [u8; 32],
    phase: FleetAdmissionRootTransitionPhase,
    authority: &FleetCoordinatorBinding,
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    generation: u64,
    policy_digest: [u8; 32],
    participant_catalog_digest: [u8; 32],
    participant_count: u32,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(ROOT_RECEIPT_DIGEST_DOMAIN);
    encoder.bytes(&operation_id);
    encoder.u8(match phase {
        FleetAdmissionRootTransitionPhase::Preparing => 0,
        FleetAdmissionRootTransitionPhase::PerimeterFenced => 1,
        FleetAdmissionRootTransitionPhase::Activating => 2,
        FleetAdmissionRootTransitionPhase::Opening => 3,
        FleetAdmissionRootTransitionPhase::Converged => 4,
        FleetAdmissionRootTransitionPhase::Released => 5,
    });
    encode_coordinator_binding(&mut encoder, authority);
    encoder.bytes(placement_subnet.as_principal().as_slice());
    encoder.bytes(fleet_subnet_root.as_slice());
    encoder.u64(generation);
    encoder.bytes(&policy_digest);
    encoder.bytes(&participant_catalog_digest);
    encoder.u32(participant_count);
    encoder.finish()
}

/// Reconstruct the exact target receipt expected for one outbound phase call.
pub fn expected_fleet_admission_target_receipt(
    operation_id: [u8; 32],
    phase: FleetAdmissionTargetTransitionPhase,
    expected_generation: u64,
    expected_policy_digest: [u8; 32],
    successor: FleetAdmissionProjection,
) -> Result<FleetAdmissionTargetReceipt, FleetAdmissionProjectionValidationError> {
    let phase_model = match phase {
        FleetAdmissionTargetTransitionPhase::Prepare => {
            FleetAdmissionTargetTransitionPhaseModel::Prepare
        }
        FleetAdmissionTargetTransitionPhase::Activate => {
            FleetAdmissionTargetTransitionPhaseModel::Activate
        }
        FleetAdmissionTargetTransitionPhase::Open => FleetAdmissionTargetTransitionPhaseModel::Open,
    };
    let request = compile_target_transition_request(
        operation_id,
        phase_model,
        expected_generation,
        expected_policy_digest,
        successor.clone(),
    )?;
    Ok(fleet_admission_target_receipt(
        &successor,
        &FleetAdmissionProjectionReceiptModel {
            operation_id,
            phase: phase_model,
            request_hash: request.request_hash,
            receipt_hash: request.receipt_hash,
        },
    ))
}

#[must_use]
pub fn fleet_admission_root_prepare_request_digest(
    request: &FleetAdmissionPrepareRootRequest,
    root: &crate::ids::FleetSubnetRootBinding,
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(ROOT_PREPARE_REQUEST_DIGEST_DOMAIN);
    encode_coordinator_binding(&mut encoder, &request.authority);
    encoder.bytes(root.placement_subnet.as_principal().as_slice());
    encoder.bytes(root.fleet_subnet_root.as_slice());
    encoder.bytes(&request.operation_id);
    encoder.u64(request.expected_generation);
    encoder.bytes(&request.expected_policy_digest);
    encoder.u64(request.successor.generation);
    encoder.bytes(&request.successor.policy_digest);
    encoder.u8(match request.stage {
        FleetAdmissionPrepareRootStage::Reserve => 0,
        FleetAdmissionPrepareRootStage::Fence => 1,
        FleetAdmissionPrepareRootStage::Release => 2,
    });
    encoder.finish()
}

fn encode_root_catalogs(
    encoder: &mut CanonicalAdmissionEncoder,
    catalogs: &[FleetAdmissionRootCatalogAuthorityModel],
) {
    encoder.u64(catalogs.len() as u64);
    for catalog in catalogs {
        encoder.bytes(catalog.fleet_subnet_root.as_slice());
        encoder.bytes(&catalog.participant_catalog_digest);
        encoder.u32(catalog.participant_count);
    }
}

fn compile_target_transition_request(
    operation_id: [u8; 32],
    phase: FleetAdmissionTargetTransitionPhaseModel,
    expected_generation: u64,
    expected_policy_digest: [u8; 32],
    successor: FleetAdmissionProjection,
) -> Result<FleetAdmissionTargetTransitionRequestModel, FleetAdmissionProjectionValidationError> {
    if operation_id == [0; 32] {
        return Err(FleetAdmissionProjectionValidationError::RetainedReceiptInvalid);
    }
    let request_hash = fleet_admission_target_transition_request_digest(
        operation_id,
        phase,
        expected_generation,
        expected_policy_digest,
        &successor,
    );
    let receipt_hash = fleet_admission_target_transition_receipt_digest(
        operation_id,
        phase,
        request_hash,
        &successor,
    );
    Ok(FleetAdmissionTargetTransitionRequestModel {
        operation_id,
        phase,
        expected_generation,
        expected_policy_digest,
        successor,
        request_hash,
        receipt_hash,
    })
}

fn fleet_admission_target_transition_request_digest(
    operation_id: [u8; 32],
    phase: FleetAdmissionTargetTransitionPhaseModel,
    expected_generation: u64,
    expected_policy_digest: [u8; 32],
    successor: &FleetAdmissionProjection,
) -> [u8; 32] {
    let mut encoder =
        CanonicalAdmissionEncoder::with_domain(TARGET_TRANSITION_REQUEST_DIGEST_DOMAIN);
    encoder.bytes(&operation_id);
    encoder.u8(phase.hash_byte());
    encoder.u64(expected_generation);
    encoder.bytes(&expected_policy_digest);
    encode_coordinator_binding(&mut encoder, &successor.authority);
    encode_managed_target(&mut encoder, &successor.target);
    encoder.u64(successor.generation);
    encoder.bytes(&successor.policy_digest);
    encoder.bytes(&successor.projection_digest);
    encoder.finish()
}

fn fleet_admission_target_transition_receipt_digest(
    operation_id: [u8; 32],
    phase: FleetAdmissionTargetTransitionPhaseModel,
    request_hash: [u8; 32],
    successor: &FleetAdmissionProjection,
) -> [u8; 32] {
    let mut encoder =
        CanonicalAdmissionEncoder::with_domain(TARGET_TRANSITION_RECEIPT_DIGEST_DOMAIN);
    encoder.bytes(&operation_id);
    encoder.u8(phase.hash_byte());
    encoder.bytes(&request_hash);
    encode_coordinator_binding(&mut encoder, &successor.authority);
    encode_managed_target(&mut encoder, &successor.target);
    encoder.u64(successor.generation);
    encoder.bytes(&successor.policy_digest);
    encoder.bytes(&successor.projection_digest);
    encoder.finish()
}

fn template_digest(fleet_principals: &[Principal], rules: &[FleetAdmissionRule]) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(TEMPLATE_DIGEST_DOMAIN);
    encoder.u16(FLEET_ADMISSION_SCHEMA_VERSION);
    encode_semantics(&mut encoder, fleet_principals, rules);
    encoder.finish()
}

fn policy_digest(
    fleet: &FleetBinding,
    generation: u64,
    fleet_principals: &[Principal],
    rules: &[FleetAdmissionRule],
) -> [u8; 32] {
    let mut encoder = CanonicalAdmissionEncoder::with_domain(POLICY_DIGEST_DOMAIN);
    encoder.u16(FLEET_ADMISSION_SCHEMA_VERSION);
    encoder.bytes(fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(fleet.fleet.fleet_id.as_bytes());
    encoder.string(fleet.app.as_str());
    encoder.u64(generation);
    encode_semantics(&mut encoder, fleet_principals, rules);
    encoder.finish()
}

fn encode_semantics(
    encoder: &mut CanonicalAdmissionEncoder,
    fleet_principals: &[Principal],
    rules: &[FleetAdmissionRule],
) {
    encoder.u64(fleet_principals.len() as u64);
    for principal in fleet_principals {
        encoder.bytes(principal.as_slice());
    }
    encoder.u64(rules.len() as u64);
    for rule in rules {
        encode_selector(encoder, &rule.selector);
        encoder.u64(rule.principals.len() as u64);
        for principal in &rule.principals {
            encoder.bytes(principal.as_slice());
        }
    }
}

fn encode_selector(encoder: &mut CanonicalAdmissionEncoder, selector: &FleetAdmissionSelector) {
    match selector {
        FleetAdmissionSelector::Fleet => encoder.u8(0),
        FleetAdmissionSelector::ComponentSpec(component_spec) => {
            encoder.u8(1);
            encoder.string(component_spec.as_str());
        }
        FleetAdmissionSelector::ComponentInstance(component_instance) => {
            encoder.u8(2);
            encoder.bytes(component_instance.as_bytes());
        }
        FleetAdmissionSelector::FleetSubnetRoot(fleet_subnet_root) => {
            encoder.u8(3);
            encoder.bytes(fleet_subnet_root.as_principal().as_slice());
        }
    }
}

fn encode_coordinator_binding(
    encoder: &mut CanonicalAdmissionEncoder,
    binding: &crate::ids::FleetCoordinatorBinding,
) {
    encoder.bytes(binding.fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(binding.fleet.fleet.fleet_id.as_bytes());
    encoder.string(binding.fleet.app.as_str());
    encoder.bytes(binding.coordinator_subnet.as_principal().as_slice());
    encoder.bytes(binding.coordinator.as_slice());
}

fn encode_managed_target(encoder: &mut CanonicalAdmissionEncoder, target: &ManagedCanisterBinding) {
    match target {
        ManagedCanisterBinding::Component(binding) => {
            encoder.u8(0);
            encode_component_binding(encoder, binding);
        }
        ManagedCanisterBinding::ComponentChild(binding) => {
            encoder.u8(1);
            encode_component_binding(encoder, &binding.component);
            encoder.bytes(binding.parent_canister_id.as_slice());
            encoder.string(binding.role.as_str());
            encoder.bytes(binding.canister_id.as_slice());
        }
    }
}

fn encode_component_binding(
    encoder: &mut CanonicalAdmissionEncoder,
    binding: &crate::ids::ComponentBinding,
) {
    encode_coordinator_binding(encoder, &binding.authority.binding);
    encoder.u64(binding.authority.epoch);
    encoder.bytes(binding.component.as_bytes());
    encoder.string(binding.component_spec.as_str());
    encoder.bytes(&binding.spec_hash);
    encoder.string(binding.role.as_str());
    encoder.bytes(binding.placement_subnet.as_principal().as_slice());
    encoder.bytes(binding.fleet_subnet_root.as_slice());
    encoder.bytes(binding.canister_id.as_slice());
}

const fn projection_target_authority(
    target: &ManagedCanisterBinding,
) -> &crate::ids::FleetCoordinatorBinding {
    match target {
        ManagedCanisterBinding::Component(binding) => &binding.authority.binding,
        ManagedCanisterBinding::ComponentChild(binding) => &binding.component.authority.binding,
    }
}

struct CanonicalAdmissionEncoder {
    bytes: Vec<u8>,
}

impl CanonicalAdmissionEncoder {
    fn with_domain(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn finish(self) -> [u8; 32] {
        Sha256::digest(self.bytes).into()
    }
}
