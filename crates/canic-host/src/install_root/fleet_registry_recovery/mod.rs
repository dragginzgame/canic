//! Module: install_root::fleet_registry_recovery
//!
//! Responsibility: validate the exact live Registry states that may resume one fresh install.
//! Does not own: Registry mutation, Component provisioning, or host journal transitions.
//! Boundary: a post-activation successor is accepted only with exact Coordinator operation proof.

use super::operations::{LiveRegistryEvidence, query_with_arg};
use crate::{icp::IcpCli, protocol_binding::ResolvedProtocolBinding};
use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorOperationStatusResponse, CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningStatusResponse,
        },
        fleet_registry::{
            FleetComponentSpecEntry, FleetRegistry, FleetRegistryManifest, FleetRegistryVersion,
            FleetSubnetRootEntry,
        },
        role::OperationStatusRequest,
    },
    ids::{FleetAdmissionPolicy, FleetRegistryAuthority},
    protocol,
};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum FleetRegistryRecoveryError {
    #[error("live Fleet Registry differs from the exact planned {0} snapshot")]
    LiveRegistryMismatch(&'static str),
}

/// Accept the all-Joining baseline or one exactly proven later fresh-install state.
pub(super) fn require_joining_or_recovered_registry(
    request: JoiningRegistryRecoveryRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if registry_evidence_matches(
        request.active.component_topology,
        request.joining,
        request.active.live,
    )? {
        return Ok(());
    }
    let successor_evidence = successor_evidence_for_live_registry(
        request.active.icp,
        request.active.binding,
        request.active.coordinator,
        request.active.active,
        request.active.live,
        request.active.expected_operation_id,
    )?;
    require_joining_or_recovered_evidence(
        request.active.component_topology,
        request.joining,
        request.active.active,
        request.active.live,
        request.active.expected_operation_id,
        request.active.expected_plan_hash,
        successor_evidence.as_ref(),
    )
}

/// Accept the exact all-Active baseline or its exact initial service-publication successor.
pub(super) fn require_active_or_service_successor_registry(
    request: ActiveRegistryRecoveryRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let successor_evidence = successor_evidence_for_live_registry(
        request.icp,
        request.binding,
        request.coordinator,
        request.active,
        request.live,
        request.expected_operation_id,
    )?;
    require_active_or_service_successor_evidence(
        request.component_topology,
        request.active,
        request.live,
        request.expected_operation_id,
        request.expected_plan_hash,
        successor_evidence.as_ref(),
    )
}

pub(super) struct ActiveRegistryRecoveryRequest<'a> {
    pub(super) icp: &'a IcpCli,
    pub(super) binding: &'a ResolvedProtocolBinding,
    pub(super) coordinator: Principal,
    pub(super) component_topology: &'a ComponentTopology,
    pub(super) active: &'a FleetRegistry,
    pub(super) live: &'a LiveRegistryEvidence,
    pub(super) expected_operation_id: [u8; 32],
    pub(super) expected_plan_hash: [u8; 32],
}

pub(super) struct JoiningRegistryRecoveryRequest<'a> {
    pub(super) active: ActiveRegistryRecoveryRequest<'a>,
    pub(super) joining: &'a FleetRegistry,
}

pub(super) fn require_joining_or_recovered_evidence(
    component_topology: &ComponentTopology,
    joining: &FleetRegistry,
    active: &FleetRegistry,
    live: &LiveRegistryEvidence,
    expected_operation_id: [u8; 32],
    expected_plan_hash: [u8; 32],
    successor_evidence: Option<&ComponentProvisioningSuccessorEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    if registry_evidence_matches(component_topology, joining, live)? {
        return Ok(());
    }
    require_active_or_service_successor_evidence(
        component_topology,
        active,
        live,
        expected_operation_id,
        expected_plan_hash,
        successor_evidence,
    )
}

pub(super) fn require_active_or_service_successor_evidence(
    component_topology: &ComponentTopology,
    active: &FleetRegistry,
    live: &LiveRegistryEvidence,
    expected_operation_id: [u8; 32],
    expected_plan_hash: [u8; 32],
    successor_evidence: Option<&ComponentProvisioningSuccessorEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    if registry_evidence_matches(component_topology, active, live)? {
        return Ok(());
    }

    FleetRegistryOps::validate(&active.authority, component_topology, &live.registry)?;
    let live_manifest =
        FleetRegistryOps::manifest(&live.registry.authority, component_topology, &live.registry)?;
    let live_version =
        FleetRegistryOps::version(&live.registry.authority, component_topology, &live.registry)?;
    let immutable_authority_matches = RegistryImmutableAuthority::from_registry(&live.registry)
        == RegistryImmutableAuthority::from_registry(active);
    let initial_service_successor_facts = [
        active.revision.checked_add(1) == Some(live.registry.revision),
        active.services.is_empty(),
        !live.registry.services.is_empty(),
    ];
    let is_initial_service_successor = initial_service_successor_facts
        .into_iter()
        .all(std::convert::identity);
    let evidence_is_exact = RegistryHead {
        manifest: &live.manifest,
        version: &live.version,
    } == RegistryHead {
        manifest: &live_manifest,
        version: &live_version,
    };
    let active_version = FleetRegistryOps::version(&active.authority, component_topology, active)?;
    let successor_is_valid = [
        immutable_authority_matches,
        is_initial_service_successor,
        evidence_is_exact,
        successor_evidence.is_some_and(|evidence| {
            evidence.operation_id == expected_operation_id
                && evidence.plan_hash == expected_plan_hash
                && evidence.source_registry == active_version
                && evidence.published_registry.as_ref() == Some(&live.version)
                && evidence.operation == FleetComponentProvisioningOperation::FreshInstall
                && component_provisioning_phase_has_published_services(evidence.phase)
        }),
    ]
    .into_iter()
    .all(std::convert::identity);
    if !successor_is_valid {
        return Err(FleetRegistryRecoveryError::LiveRegistryMismatch(
            "all-Joining, all-Active or exact Fleet-service successor",
        )
        .into());
    }
    Ok(())
}

fn registry_evidence_matches(
    component_topology: &ComponentTopology,
    expected: &FleetRegistry,
    live: &LiveRegistryEvidence,
) -> Result<bool, Box<dyn std::error::Error>> {
    let expected_manifest =
        FleetRegistryOps::manifest(&expected.authority, component_topology, expected)?;
    let expected_version =
        FleetRegistryOps::version(&expected.authority, component_topology, expected)?;
    Ok(RegistryEvidence::from_live(live)
        == (RegistryEvidence {
            registry: expected,
            manifest: &expected_manifest,
            version: &expected_version,
        }))
}

fn query_component_provisioning_successor_evidence(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
    operation_id: [u8; 32],
) -> Result<ComponentProvisioningSuccessorEvidence, Box<dyn std::error::Error>> {
    let response = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
    )?;
    let CoordinatorStatusResponse::Operation(
        CoordinatorOperationStatusResponse::ComponentProvisioning(status),
    ) = response
    else {
        return Err(FleetRegistryRecoveryError::LiveRegistryMismatch(
            "service-successor provisioning evidence",
        )
        .into());
    };
    Ok(ComponentProvisioningSuccessorEvidence::from(&status))
}

fn successor_evidence_for_live_registry(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
    active: &FleetRegistry,
    live: &LiveRegistryEvidence,
    operation_id: [u8; 32],
) -> Result<Option<ComponentProvisioningSuccessorEvidence>, Box<dyn std::error::Error>> {
    if live.registry == *active {
        return Ok(None);
    }
    query_component_provisioning_successor_evidence(icp, binding, coordinator, operation_id)
        .map(Some)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ComponentProvisioningSuccessorEvidence {
    pub(super) operation_id: [u8; 32],
    pub(super) plan_hash: [u8; 32],
    pub(super) source_registry: FleetRegistryVersion,
    pub(super) published_registry: Option<FleetRegistryVersion>,
    pub(super) operation: FleetComponentProvisioningOperation,
    pub(super) phase: FleetComponentProvisioningPhase,
}

impl From<&FleetComponentProvisioningStatusResponse> for ComponentProvisioningSuccessorEvidence {
    fn from(status: &FleetComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: status.operation_id,
            plan_hash: status.plan_hash,
            source_registry: status.fleet_registry.clone(),
            published_registry: status.published_fleet_registry.clone(),
            operation: status.operation.clone(),
            phase: status.phase,
        }
    }
}

const fn component_provisioning_phase_has_published_services(
    phase: FleetComponentProvisioningPhase,
) -> bool {
    matches!(
        phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
            | FleetComponentProvisioningPhase::ConfirmingDirectories
            | FleetComponentProvisioningPhase::DirectoriesConfirmed
            | FleetComponentProvisioningPhase::ActivatingRuntimes
            | FleetComponentProvisioningPhase::RuntimesActivated
    )
}

#[derive(Eq, PartialEq)]
struct RegistryEvidence<'a> {
    registry: &'a FleetRegistry,
    manifest: &'a FleetRegistryManifest,
    version: &'a FleetRegistryVersion,
}

impl<'a> RegistryEvidence<'a> {
    const fn from_live(live: &'a LiveRegistryEvidence) -> Self {
        Self {
            registry: &live.registry,
            manifest: &live.manifest,
            version: &live.version,
        }
    }
}

#[derive(Eq, PartialEq)]
struct RegistryHead<'a> {
    manifest: &'a FleetRegistryManifest,
    version: &'a FleetRegistryVersion,
}

#[derive(Eq, PartialEq)]
struct RegistryImmutableAuthority<'a> {
    authority: &'a FleetRegistryAuthority,
    admission: &'a FleetAdmissionPolicy,
    component_specs: &'a [FleetComponentSpecEntry],
    fleet_subnet_roots: &'a [FleetSubnetRootEntry],
}

impl<'a> RegistryImmutableAuthority<'a> {
    fn from_registry(registry: &'a FleetRegistry) -> Self {
        Self {
            authority: &registry.authority,
            admission: &registry.admission,
            component_specs: &registry.component_specs,
            fleet_subnet_roots: &registry.fleet_subnet_roots,
        }
    }
}
