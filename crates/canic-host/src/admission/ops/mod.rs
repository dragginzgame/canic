//! Fleet admission ops.
//!
//! Host owns authority and operations; command parsing and rendering belong to CLI.

#[cfg(test)]
mod tests;

use crate::{
    admission::{
        ADMISSION_PLAN_SCHEMA_VERSION, AdmissionError, authority_error,
        model::{
            AdmissionConnection, AdmissionMutation, AdmissionParticipantCatalogRecord,
            AdmissionPlanRecord,
        },
        view::AdmissionRootReport,
    },
    fleet_ensure::CurrentFleetResolution,
    icp::IcpCli,
};
use canic_contracts::{
    dto::{
        fleet_admission::{
            FleetAdmissionMutationAction, FleetAdmissionMutationRequest,
            FleetAdmissionOperationPhase, FleetAdmissionRootParticipantPhase,
            FleetAdmissionRootParticipantStatus, FleetAdmissionRootStatusResponse,
            FleetAdmissionRootTransitionPhase, FleetAdmissionStatusResponse,
        },
        fleet_registry::{FleetRegistry, FleetRegistryVersion, FleetSubnetRootStatus},
        page::PageRequest,
    },
    ids::{FleetAdmissionPolicy, FleetAdmissionSelector, ManagedCanisterBinding},
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    shared_support::{
        fleet_admission_authority::{
            FleetAdmissionMutationActionModel, FleetAdmissionMutationOperationInput,
            FleetAdmissionRootCatalogAuthorityModel, fleet_admission_mutation_operation_id,
            mutate_fleet_admission_membership,
        },
        fleet_admission_policy::{
            compile_installed_fleet_admission_policy, effective_fleet_admission_principals,
            fleet_admission_participant_catalog_digest,
            fleet_admission_root_participant_catalog_digest, fleet_admission_target_for_binding,
            materialize_fleet_admission_projection,
        },
        fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS,
    },
};
use ic_platform::principal::ExternalPrincipal as Principal;

pub(super) fn validate_plan_file(
    fleet: &str,
    environment: &str,
    plan: &AdmissionPlanRecord,
) -> Result<(), AdmissionError> {
    let expected = validate_plan_semantics(plan)?;
    if plan.schema_version != ADMISSION_PLAN_SCHEMA_VERSION
        || plan.fleet != fleet
        || plan.environment != environment
        || expected != plan.successor_policy
    {
        return Err(authority_error(
            "plan identity, environment, or successor policy is invalid",
        ));
    }
    let catalog_authorities = participant_catalog_authorities(&plan.participant_catalogs)?;
    let (participant_catalog_digest, participant_count) =
        aggregate_participant_catalog_authority(&catalog_authorities)?;
    let operation_id = fleet_admission_mutation_operation_id(
        &plan.predecessor_registry,
        &FleetAdmissionMutationOperationInput {
            expected_generation: plan.request.expected_generation,
            expected_policy_digest: plan.request.expected_policy_digest,
            action: action_model(plan.request.action),
            selector: plan.request.selector.clone(),
            principal: plan.request.principal,
            successor_policy_digest: plan.request.successor_policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    );
    if plan.coordinator != plan.predecessor_registry.authority.binding.coordinator
        || plan.request.authority != plan.predecessor_registry.authority.binding
        || plan.request.expected_generation != plan.predecessor_policy.generation
        || plan.request.expected_policy_digest != plan.predecessor_policy.policy_digest
        || plan.request.successor_policy_digest != plan.successor_policy.policy_digest
        || plan.request.participant_catalog_digest != participant_catalog_digest
        || plan.request.participant_count != participant_count
        || plan.request.operation_id != operation_id
    {
        return Err(authority_error(
            "plan Registry, request, or operation identity is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_plan_semantics(
    plan: &AdmissionPlanRecord,
) -> Result<FleetAdmissionPolicy, AdmissionError> {
    let membership = mutate_fleet_admission_membership(
        &plan.predecessor_policy,
        action_model(plan.request.action),
        &plan.request.selector,
        plan.request.principal,
    )
    .map_err(|error| AdmissionError::InvalidMutation(error.to_string()))?;
    let generation = if membership.changed {
        plan.predecessor_policy
            .generation
            .checked_add(1)
            .ok_or_else(|| AdmissionError::InvalidMutation("generation is exhausted".to_string()))?
    } else {
        plan.predecessor_policy.generation
    };
    compile_installed_fleet_admission_policy(
        plan.predecessor_policy.fleet.clone(),
        generation,
        membership.fleet_principals,
        membership.rules,
    )
    .map_err(|error| AdmissionError::InvalidMutation(error.to_string()))
}

pub(super) fn validate_live_plan_state(
    admission: &FleetAdmissionStatusResponse,
    plan: &AdmissionPlanRecord,
    predecessor_matches: bool,
) -> Result<(), AdmissionError> {
    if exact_retained_operation(admission, plan) {
        return Ok(());
    }
    if predecessor_matches {
        return require_idle_admission(admission);
    }
    Err(authority_error(
        "plan no longer matches the live Registry, participants, or retained operation",
    ))
}

pub(super) fn exact_retained_operation(
    admission: &FleetAdmissionStatusResponse,
    plan: &AdmissionPlanRecord,
) -> bool {
    admission
        .current_operation
        .as_ref()
        .or(admission.last_result.as_ref())
        .is_some_and(|operation| {
            operation.operation_id == plan.request.operation_id
                && operation.action == plan.request.action
                && operation.selector == plan.request.selector
                && operation.principal == plan.request.principal
                && operation_successor(operation)
                    == (
                        plan.successor_policy.generation,
                        plan.successor_policy.policy_digest,
                    )
        })
}

pub(super) fn validate_live_authority(
    initial: &FleetRegistry,
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    admission: &FleetAdmissionStatusResponse,
) -> Result<(), AdmissionError> {
    let fleet = &registry.authority.binding.fleet;
    let narrower_references = registry
        .admission
        .rules
        .iter()
        .map(|rule| rule.principals.len())
        .sum::<usize>();
    let registry_policy_matches_status = if admission.active.generation
        == registry.admission.generation
        && admission.active.policy_digest == registry.admission.policy_digest
    {
        usize::from(admission.active.fleet_principal_count)
            == registry.admission.fleet_principals.len()
            && usize::from(admission.active.narrower_rule_count) == registry.admission.rules.len()
            && usize::from(admission.active.narrower_principal_reference_count)
                == narrower_references
    } else {
        admission
            .current_operation
            .as_ref()
            .is_some_and(|operation| {
                operation_successor_status(operation).is_some_and(|successor| {
                    successor.generation == registry.admission.generation
                        && successor.policy_digest == registry.admission.policy_digest
                        && usize::from(successor.fleet_principal_count)
                            == registry.admission.fleet_principals.len()
                        && usize::from(successor.narrower_rule_count)
                            == registry.admission.rules.len()
                        && usize::from(successor.narrower_principal_reference_count)
                            == narrower_references
                })
            })
    };
    let exact = registry.authority == version.authority
        && registry.revision == version.revision
        && registry.authority == initial.authority
        && registry.fleet_subnet_roots == initial.fleet_subnet_roots
        && admission.fleet == fleet.clone()
        && registry_policy_matches_status;
    if exact {
        Ok(())
    } else {
        Err(authority_error(
            "live Coordinator Registry and admission status disagree with current ensure authority",
        ))
    }
}

pub(super) fn query_coordinator_registry(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<FleetRegistry, AdmissionError> {
    Ok(crate::registry_discovery::snapshot(icp, coordinator)?)
}

pub(super) fn query_coordinator_registry_version(
    icp: &IcpCli,
    current: &CurrentFleetResolution,
    root: &std::path::Path,
    coordinator: Principal,
) -> Result<FleetRegistryVersion, AdmissionError> {
    let entry = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == coordinator.to_text())
        .ok_or_else(|| authority_error("current inventory omits Coordinator"))?;
    let schema =
        crate::protocol_binding::resolve_current_registry_wire_schema(root, current, entry)?;
    Ok(crate::registry_discovery::version(
        icp,
        &schema,
        coordinator,
    )?)
}

pub(super) fn current_admission_schema(
    current: &CurrentFleetResolution,
    root: &std::path::Path,
    principal: Principal,
) -> Result<crate::release_set::BuiltWireSchema, AdmissionError> {
    let entry = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == principal.to_text())
        .ok_or_else(|| authority_error("current inventory omits admission participant"))?;
    Ok(crate::protocol_binding::resolve_current_registry_wire_schema(root, current, entry)?)
}

pub(super) fn query_coordinator_admission(
    icp: &IcpCli,
    _schema: &crate::release_set::BuiltWireSchema,
    coordinator: Principal,
) -> Result<FleetAdmissionStatusResponse, AdmissionError> {
    use canic_contracts::{dto::admission_control, protocol::admission_control as protocol};
    let response: admission_control::StatusResponse = crate::canister_protocol::query_wire(
        icp,
        coordinator,
        protocol::status::METHOD,
        protocol::status::METHOD_ID,
        &admission_control::StatusRequest {
            selector: canic_contracts::dto::fleet_snapshot::FleetAdmissionSelector::Fleet,
            page: PageRequest {
                limit: 1,
                offset: 0,
            },
        },
        protocol::REQUEST_LIMITS,
        protocol::RESPONSE_LIMITS,
    )?;
    Ok(response.into())
}

pub(super) fn participant_catalog_authorities(
    catalogs: &[AdmissionParticipantCatalogRecord],
) -> Result<Vec<FleetAdmissionRootCatalogAuthorityModel>, AdmissionError> {
    let catalogs_are_canonical = !catalogs.is_empty()
        && catalogs.windows(2).all(|pair| {
            pair[0].fleet_subnet_root.as_slice() < pair[1].fleet_subnet_root.as_slice()
        });
    let total = catalogs.iter().try_fold(0_usize, |total, catalog| {
        total.checked_add(catalog.participants.len())
    });
    let participants_are_canonical = catalogs.iter().all(|catalog| {
        catalog.participant_catalog_digest != [0; 32]
            && catalog.participants.len() <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
            && catalog
                .participants
                .iter()
                .all(|target| target_root(target) == catalog.fleet_subnet_root)
            && catalog.participants.windows(2).all(|pair| {
                target_principal(&pair[0]).as_slice() < target_principal(&pair[1]).as_slice()
            })
    });
    if !catalogs_are_canonical
        || !participants_are_canonical
        || total.is_none_or(|total| total > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
    {
        return Err(authority_error(
            "plan participant catalogs are noncanonical or exceed the protocol bound",
        ));
    }
    catalogs
        .iter()
        .map(|catalog| {
            let participant_count = u32::try_from(catalog.participants.len()).map_err(|_| {
                authority_error("Root participant count exceeds the protocol bound")
            })?;
            Ok(FleetAdmissionRootCatalogAuthorityModel {
                fleet_subnet_root: catalog.fleet_subnet_root,
                participant_catalog_digest: catalog.participant_catalog_digest,
                participant_count,
            })
        })
        .collect()
}

pub(super) fn aggregate_participant_catalog_authority(
    catalogs: &[FleetAdmissionRootCatalogAuthorityModel],
) -> Result<([u8; 32], u32), AdmissionError> {
    let participant_count = catalogs.iter().try_fold(0_u32, |total, catalog| {
        total.checked_add(catalog.participant_count)
    });
    let participant_count = participant_count
        .filter(|count| {
            usize::try_from(*count)
                .is_ok_and(|count| count <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
        })
        .ok_or_else(|| authority_error("participant catalog count exceeds the protocol bound"))?;
    Ok((
        fleet_admission_participant_catalog_digest(catalogs),
        participant_count,
    ))
}

pub(super) fn root_report(
    root: Principal,
    status: &FleetAdmissionRootStatusResponse,
    participants: &[FleetAdmissionRootParticipantStatus],
) -> AdmissionRootReport {
    let count = |phase| {
        u64::try_from(
            participants
                .iter()
                .filter(|participant| participant.phase == phase)
                .count(),
        )
        .unwrap_or(u64::MAX)
    };
    AdmissionRootReport {
        root,
        active_generation: status.active_generation,
        active_policy_digest: hex_bytes(status.active_policy_digest),
        operation_id: status.operation_id.map(hex_bytes),
        phase: status.phase.map(root_phase_label).map(str::to_string),
        participant_count: status.participants.total,
        pending_count: count(FleetAdmissionRootParticipantPhase::Pending),
        prepared_count: count(FleetAdmissionRootParticipantPhase::Prepared),
        activated_count: count(FleetAdmissionRootParticipantPhase::Activated),
        open_count: count(FleetAdmissionRootParticipantPhase::Open),
        first_unresolved: participants
            .iter()
            .find(|participant| participant.phase != FleetAdmissionRootParticipantPhase::Open)
            .map(|participant| target_principal(&participant.target)),
    }
}

pub(super) fn same_root_status_head(
    left: &FleetAdmissionRootStatusResponse,
    right: &FleetAdmissionRootStatusResponse,
) -> bool {
    left.operation_id == right.operation_id
        && left.phase == right.phase
        && left.active_generation == right.active_generation
        && left.active_policy_digest == right.active_policy_digest
        && left.successor_generation == right.successor_generation
        && left.successor_policy_digest == right.successor_policy_digest
        && left.participant_catalog_digest == right.participant_catalog_digest
        && left.participants.total == right.participants.total
}

pub(super) fn require_idle_admission(
    status: &FleetAdmissionStatusResponse,
) -> Result<(), AdmissionError> {
    if status.current_operation.is_some() {
        Err(authority_error(
            "another Fleet-admission mutation is active",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_selector_exists(
    registry: &FleetRegistry,
    selector: &FleetAdmissionSelector,
) -> Result<(), AdmissionError> {
    let exists = match selector {
        FleetAdmissionSelector::Fleet => true,
        FleetAdmissionSelector::ComponentSpec(spec) => registry
            .component_specs
            .iter()
            .any(|entry| &entry.component_spec == spec),
        FleetAdmissionSelector::ComponentInstance(component) => {
            registry.services.iter().any(|service| {
                service
                    .members
                    .iter()
                    .any(|member| &member.component == component)
            })
        }
        FleetAdmissionSelector::FleetSubnetRoot(subnet) => {
            registry.fleet_subnet_roots.iter().any(|root| {
                &root.placement_subnet == subnet && root.status == FleetSubnetRootStatus::Active
            })
        }
    };
    if exists {
        Ok(())
    } else {
        Err(AdmissionError::InvalidSelector(
            "selector is absent from the active Fleet Registry".to_string(),
        ))
    }
}

pub(super) const fn action_model(
    action: FleetAdmissionMutationAction,
) -> FleetAdmissionMutationActionModel {
    match action {
        FleetAdmissionMutationAction::Add => FleetAdmissionMutationActionModel::Add,
        FleetAdmissionMutationAction::Remove => FleetAdmissionMutationActionModel::Remove,
    }
}

pub(super) fn operation_identity(
    operation: &canic_contracts::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> (String, String) {
    (
        hex_bytes(operation.operation_id),
        operation_phase_label(&operation.phase).to_string(),
    )
}

pub(super) const fn operation_successor(
    operation: &canic_contracts::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> (u64, [u8; 32]) {
    match &operation.phase {
        FleetAdmissionOperationPhase::Planned { successor }
        | FleetAdmissionOperationPhase::Preparing { successor }
        | FleetAdmissionOperationPhase::Releasing { successor }
        | FleetAdmissionOperationPhase::PerimeterFenced { successor }
        | FleetAdmissionOperationPhase::Activating { successor }
        | FleetAdmissionOperationPhase::Opening { successor } => {
            (successor.generation, successor.policy_digest)
        }
        FleetAdmissionOperationPhase::Completed(response) => {
            (response.generation, response.policy_digest)
        }
    }
}

pub(super) const fn operation_successor_status(
    operation: &canic_contracts::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> Option<&canic_contracts::dto::fleet_admission::FleetAdmissionPolicyStatus> {
    match &operation.phase {
        FleetAdmissionOperationPhase::Planned { successor }
        | FleetAdmissionOperationPhase::Preparing { successor }
        | FleetAdmissionOperationPhase::Releasing { successor }
        | FleetAdmissionOperationPhase::PerimeterFenced { successor }
        | FleetAdmissionOperationPhase::Activating { successor }
        | FleetAdmissionOperationPhase::Opening { successor } => Some(successor),
        FleetAdmissionOperationPhase::Completed(_) => None,
    }
}

pub(super) const fn operation_phase_label(phase: &FleetAdmissionOperationPhase) -> &'static str {
    match phase {
        FleetAdmissionOperationPhase::Planned { .. } => "planned",
        FleetAdmissionOperationPhase::Preparing { .. } => "preparing",
        FleetAdmissionOperationPhase::Releasing { .. } => "releasing",
        FleetAdmissionOperationPhase::PerimeterFenced { .. } => "perimeter_fenced",
        FleetAdmissionOperationPhase::Activating { .. } => "activating",
        FleetAdmissionOperationPhase::Opening { .. } => "opening",
        FleetAdmissionOperationPhase::Completed(_) => "completed",
    }
}

pub(super) const fn root_phase_label(phase: FleetAdmissionRootTransitionPhase) -> &'static str {
    match phase {
        FleetAdmissionRootTransitionPhase::Preparing => "preparing",
        FleetAdmissionRootTransitionPhase::PerimeterFenced => "perimeter_fenced",
        FleetAdmissionRootTransitionPhase::Activating => "activating",
        FleetAdmissionRootTransitionPhase::Opening => "opening",
        FleetAdmissionRootTransitionPhase::Converged => "converged",
        FleetAdmissionRootTransitionPhase::Released => "released",
    }
}

pub(super) const fn target_principal(
    target: &canic_contracts::ids::ManagedCanisterBinding,
) -> Principal {
    match target {
        canic_contracts::ids::ManagedCanisterBinding::Component(binding) => binding.canister_id,
        canic_contracts::ids::ManagedCanisterBinding::ComponentChild(binding) => {
            binding.canister_id
        }
    }
}

pub(super) const fn target_root(target: &ManagedCanisterBinding) -> Principal {
    match target {
        ManagedCanisterBinding::Component(component) => component.fleet_subnet_root,
        ManagedCanisterBinding::ComponentChild(child) => child.component.fleet_subnet_root,
    }
}

pub(super) fn successor_policy(
    connection: &AdmissionConnection,
    action: FleetAdmissionMutationAction,
    selector: &FleetAdmissionSelector,
    principal: Principal,
) -> Result<FleetAdmissionPolicy, AdmissionError> {
    require_selector_exists(&connection.registry, selector)?;
    let action_model = action_model(action);
    let predecessor = &connection.registry.admission;
    let membership =
        mutate_fleet_admission_membership(predecessor, action_model, selector, principal)
            .map_err(|error| AdmissionError::InvalidMutation(error.to_string()))?;
    let generation = if membership.changed {
        predecessor
            .generation
            .checked_add(1)
            .ok_or_else(|| AdmissionError::InvalidMutation("generation is exhausted".to_string()))?
    } else {
        predecessor.generation
    };
    let successor = compile_installed_fleet_admission_policy(
        predecessor.fleet.clone(),
        generation,
        membership.fleet_principals,
        membership.rules,
    )
    .map_err(|error| AdmissionError::InvalidMutation(error.to_string()))?;
    Ok(successor)
}

pub(super) fn assemble_plan(
    fleet: &str,
    environment: &str,
    connection: &AdmissionConnection,
    mutation: AdmissionMutation,
    successor: FleetAdmissionPolicy,
    participant_catalogs: Vec<AdmissionParticipantCatalogRecord>,
) -> Result<AdmissionPlanRecord, AdmissionError> {
    let AdmissionMutation {
        action,
        selector,
        principal,
    } = mutation;
    let predecessor = &connection.registry.admission;
    let catalog_authorities = participant_catalog_authorities(&participant_catalogs)?;
    let (participant_catalog_digest, participant_count) =
        aggregate_participant_catalog_authority(&catalog_authorities)?;
    let operation_id = fleet_admission_mutation_operation_id(
        &connection.registry_version,
        &FleetAdmissionMutationOperationInput {
            expected_generation: predecessor.generation,
            expected_policy_digest: predecessor.policy_digest,
            action: action_model(action),
            selector: selector.clone(),
            principal,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    );
    Ok(AdmissionPlanRecord {
        schema_version: ADMISSION_PLAN_SCHEMA_VERSION,
        fleet: fleet.to_string(),
        environment: environment.to_string(),
        coordinator: connection.coordinator,
        predecessor_registry: connection.registry_version.clone(),
        predecessor_policy: predecessor.clone(),
        successor_policy: successor.clone(),
        participant_catalogs,
        request: FleetAdmissionMutationRequest {
            authority: connection.registry.authority.binding.clone(),
            expected_generation: predecessor.generation,
            expected_policy_digest: predecessor.policy_digest,
            action,
            selector,
            principal,
            operation_id,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    })
}

pub(super) fn submit(
    connection: &AdmissionConnection,
    plan: &AdmissionPlanRecord,
) -> Result<canic_contracts::dto::fleet_admission::FleetAdmissionMutationResponse, AdmissionError> {
    use canic_contracts::protocol::admission_control as protocol;
    let response: canic_contracts::dto::fleet_admission::FleetAdmissionMutationResponse =
        crate::canister_protocol::call_wire(
            &connection.icp,
            connection.coordinator,
            protocol::mutate::METHOD,
            protocol::mutate::METHOD_ID,
            &canic_contracts::dto::admission_control::MutationRequest::from(plan.request.clone()),
            protocol::REQUEST_LIMITS,
            protocol::RESPONSE_LIMITS,
        )?;
    if response.operation_id != plan.request.operation_id
        || response.generation != plan.successor_policy.generation
        || response.policy_digest != plan.successor_policy.policy_digest
    {
        return Err(authority_error(
            "Coordinator mutation response differs from the accepted plan",
        ));
    }
    Ok(response)
}

pub(super) fn project_participant_catalog(
    fleet_subnet_root: Principal,
    status: FleetAdmissionRootStatusResponse,
    participants: Vec<FleetAdmissionRootParticipantStatus>,
    successor: &FleetAdmissionPolicy,
) -> Result<AdmissionParticipantCatalogRecord, AdmissionError> {
    status
        .participant_catalog_digest
        .filter(|digest| digest != &[0; 32])
        .ok_or_else(|| authority_error("Root returned no live participant catalog"))?;
    if status.operation_id.is_some()
        || status.phase.is_some()
        || participants
            .iter()
            .any(|participant| participant.phase != FleetAdmissionRootParticipantPhase::Open)
        || usize::try_from(status.participants.total).ok() != Some(participants.len())
    {
        return Err(authority_error(
            "Root participant catalog is incomplete or not converged",
        ));
    }
    let targets = participants
        .into_iter()
        .map(|participant| participant.target)
        .collect::<Vec<_>>();
    let projections = targets
        .iter()
        .map(|target| {
            let selector = fleet_admission_target_for_binding(target);
            let principals = effective_fleet_admission_principals(successor, &selector);
            materialize_fleet_admission_projection(successor, target.clone(), principals)
                .map_err(|error| authority_error(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AdmissionParticipantCatalogRecord {
        fleet_subnet_root,
        participant_catalog_digest: fleet_admission_root_participant_catalog_digest(&projections),
        participants: targets,
    })
}

/// Read a page only after its Root has been admitted against the retained release.
pub(super) fn query_root_admission(
    icp: &IcpCli,
    _schema: &crate::release_set::BuiltWireSchema,
    root: Principal,
    page: PageRequest,
) -> Result<FleetAdmissionRootStatusResponse, AdmissionError> {
    use canic_contracts::protocol::admission_control as protocol;
    let response: canic_contracts::dto::admission_control::RootStatus =
        crate::canister_protocol::query_wire(
            icp,
            root,
            protocol::root_status::METHOD,
            protocol::root_status::METHOD_ID,
            &page,
            protocol::REQUEST_LIMITS,
            protocol::RESPONSE_LIMITS,
        )?;
    Ok(response.into())
}
