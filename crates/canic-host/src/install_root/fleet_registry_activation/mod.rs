//! Module: install_root::fleet_registry_activation
//!
//! Responsibility: atomically activate and independently verify the complete acknowledged Registry.
//! Does not own: final root mirror/Directory publication, root runtime activation, or Fleet catalog.
//! Boundary: host recovery journals exact intent before the Coordinator mutation.

#[cfg(test)]
mod tests;

use super::{
    fleet_component_provisioning_plan::{
        CompileFleetComponentProvisioningPlanRequest, compile_fleet_component_provisioning_plan,
    },
    fleet_registry_activation_journal::{
        FleetRegistryActivationPhase, PlanFleetRegistryActivationRequest,
        ResolvedFleetRegistryActivation, begin_registry_activation, plan_fleet_registry_activation,
        record_registry_activated, record_registry_activation_verified,
    },
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        expected_registry_join_entry, plan_fleet_subnet_root_install,
    },
    icp_context::InstallIcpContext,
    operations::{
        LiveRegistryEvidence, call_with_arg, fleet_registry_authority, query_live_registry,
        query_with_arg, resolve_install_protocol_binding,
    },
};
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    icp::IcpCli,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureRole,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use std::path::Path;

use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorCommand, CoordinatorCommandResponse, CoordinatorOperationStatusResponse,
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{
        FleetComponentSpecEntry, FleetRegistry, FleetRegistryManifest, FleetRegistryVersion,
        FleetSubnetRootEntry, FleetSubnetRootSnapshotAcknowledgement,
    },
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningStatusResponse,
        },
        role::OperationStatusRequest,
    },
    ids::FleetRegistryAuthority,
    protocol,
};
use thiserror::Error as ThisError;

const MAX_ACTIVATION_TRANSITIONS: usize = 4;

#[derive(Debug, ThisError)]
enum FleetRegistryActivationError {
    #[error("root Registry activation requires RegistrySyncVerified, observed {0:?}")]
    RootNotSynchronized(FleetSubnetRootInstallPhase),

    #[error("planned all-Joining Registry differs from the verified synchronization version")]
    JoiningVersionMismatch,

    #[error("live Fleet Registry differs from the exact planned {0} snapshot")]
    LiveRegistryMismatch(&'static str),

    #[error("Coordinator acknowledgement set differs from the complete planned root set")]
    AcknowledgementSetMismatch,

    #[error("Fleet Registry activation exceeded its bounded journal transitions")]
    TransitionBoundExceeded,
}

pub(super) struct VerifiedFleetRegistryActivation {
    pub registry: FleetRegistry,
    pub version: FleetRegistryVersion,
}

pub(super) struct ActivateFleetRegistryRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub joining_version: FleetRegistryVersion,
}

#[expect(
    clippy::too_many_lines,
    reason = "one closed host sequence compiles, drives and independently verifies Registry activation"
)]
pub(super) fn activate_and_verify_fleet_registry(
    request: ActivateFleetRegistryRequest<'_>,
) -> Result<VerifiedFleetRegistryActivation, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        request.icp.root(),
        request.fleet_install_plan.plan.release_build_id,
    )?;
    let authority = fleet_registry_authority(request.fleet_install_plan, request.coordinator);
    let protocol_binding = resolve_install_protocol_binding(
        request.icp,
        &infrastructure_manifest,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let mut joining_registry = FleetRegistryOps::compile_genesis(
        &request.fleet_install_plan.plan.fleet.app,
        authority.clone(),
        &component_topology,
        request.fleet_install_plan.plan.admission.clone(),
    )?;
    let mut expected_roots =
        Vec::with_capacity(request.fleet_install_plan.plan.fleet_subnet_roots.len());
    for root_plan in &request.fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator: request.coordinator,
            install_operation_id: request.install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        if !matches!(
            current.journal.phase,
            FleetSubnetRootInstallPhase::RegistrySyncVerified
                | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
                | FleetSubnetRootInstallPhase::RegistryMirrorActivated
                | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
                | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
                | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
                | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
        ) {
            return Err(
                FleetRegistryActivationError::RootNotSynchronized(current.journal.phase).into(),
            );
        }
        if current
            .journal
            .registry_sync_request
            .as_ref()
            .is_none_or(|sync| sync.expected_registry != request.joining_version)
        {
            return Err(FleetRegistryActivationError::JoiningVersionMismatch.into());
        }
        let entry = expected_registry_join_entry(&current.journal)?;
        expected_roots.push(entry.fleet_subnet_root);
        joining_registry = FleetRegistryOps::compile_joining(
            &authority,
            &component_topology,
            &joining_registry,
            entry,
        )?;
    }
    let planned_joining_version =
        FleetRegistryOps::version(&authority, &component_topology, &joining_registry)?;
    if planned_joining_version != request.joining_version {
        return Err(FleetRegistryActivationError::JoiningVersionMismatch.into());
    }

    let planned = plan_fleet_registry_activation(PlanFleetRegistryActivationRequest {
        fleet_install_plan: request.fleet_install_plan,
        component_topology: component_topology.clone(),
        joining_registry,
    })?;
    let icp = request.icp.cli();
    let current = drive_activation(
        icp,
        &protocol_binding,
        request.coordinator,
        &component_topology,
        expected_roots,
        planned,
    )?;
    let live = query_live_registry(icp, &protocol_binding, request.coordinator)?;
    let compiled =
        compile_fleet_component_provisioning_plan(CompileFleetComponentProvisioningPlanRequest {
            config: config.model(),
            fleet_install_plan: &request.fleet_install_plan.plan,
            registry: &current.journal.active_registry,
            operation_id: super::root_component_provisioning_operation_id(
                request.install_operation_id,
            ),
        })?;
    let successor_evidence = if live.registry == current.journal.active_registry {
        None
    } else {
        Some(query_component_provisioning_successor_evidence(
            icp,
            &protocol_binding,
            request.coordinator,
            compiled.prepare_request.operation_id,
        )?)
    };
    require_exact_or_service_successor_registry(
        &component_topology,
        &current.journal.active_registry,
        &live,
        compiled.prepare_request.operation_id,
        compiled.plan_hash,
        successor_evidence.as_ref(),
    )?;
    let version = FleetRegistryOps::version(
        &current.journal.active_registry.authority,
        &component_topology,
        &current.journal.active_registry,
    )?;
    Ok(VerifiedFleetRegistryActivation {
        registry: current.journal.active_registry,
        version,
    })
}

fn drive_activation(
    icp: &IcpCli,
    binding: &crate::protocol_binding::ResolvedProtocolBinding,
    coordinator: Principal,
    component_topology: &ComponentTopology,
    mut expected_roots: Vec<Principal>,
    mut current: ResolvedFleetRegistryActivation,
) -> Result<ResolvedFleetRegistryActivation, Box<dyn std::error::Error>> {
    for _ in 0..MAX_ACTIVATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetRegistryActivationPhase::Planned => {
                let live = query_live_registry(icp, binding, coordinator)?;
                require_exact_registry(
                    component_topology,
                    &current.journal.joining_registry,
                    &live,
                    "pre-activation all-Joining",
                )?;
                require_exact_acknowledgements(
                    icp,
                    binding,
                    coordinator,
                    &mut expected_roots,
                    &current.journal.request.expected_registry,
                )?;
                begin_registry_activation(&current)?
            }
            FleetRegistryActivationPhase::ActivationInFlight => {
                let response: CoordinatorCommandResponse = call_with_arg(
                    icp,
                    binding,
                    coordinator,
                    protocol::CANIC_COMMAND,
                    &CoordinatorCommand::ActivateRegistry(current.journal.request.clone()),
                )?;
                let CoordinatorCommandResponse::ActivateRegistry(response) = response else {
                    return Err(FleetRegistryActivationError::LiveRegistryMismatch(
                        "activation response correlation",
                    )
                    .into());
                };
                record_registry_activated(&current, response)?
            }
            FleetRegistryActivationPhase::Activated => {
                let live = query_live_registry(icp, binding, coordinator)?;
                require_exact_registry(
                    component_topology,
                    &current.journal.active_registry,
                    &live,
                    "post-activation all-Active",
                )?;
                record_registry_activation_verified(&current, live.manifest, live.version)?
            }
            FleetRegistryActivationPhase::Verified => return Ok(current),
        };
    }
    Err(FleetRegistryActivationError::TransitionBoundExceeded.into())
}

fn require_exact_acknowledgements(
    icp: &IcpCli,
    binding: &crate::protocol_binding::ResolvedProtocolBinding,
    coordinator: Principal,
    expected_roots: &mut [Principal],
    version: &FleetRegistryVersion,
) -> Result<(), Box<dyn std::error::Error>> {
    let live = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RootAcknowledgements,
    )?;
    let CoordinatorStatusResponse::RootAcknowledgements(live) = live else {
        return Err("Coordinator returned an unrelated root-acknowledgement response".into());
    };
    expected_roots.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));
    let cardinality_matches = live.len() == expected_roots.len();
    let entries_match = live
        .iter()
        .zip(expected_roots)
        .all(|(ack, root)| acknowledgement_matches(ack, *root, version));
    let acknowledgement_set_is_exact = [cardinality_matches, entries_match]
        .into_iter()
        .all(std::convert::identity);
    if !acknowledgement_set_is_exact {
        return Err(FleetRegistryActivationError::AcknowledgementSetMismatch.into());
    }
    Ok(())
}

fn acknowledgement_matches(
    acknowledgement: &FleetSubnetRootSnapshotAcknowledgement,
    root: Principal,
    version: &FleetRegistryVersion,
) -> bool {
    let expected = AcknowledgementAuthority { root, version };
    let observed = AcknowledgementAuthority {
        root: acknowledgement.fleet_subnet_root,
        version: &acknowledgement.version,
    };
    observed == expected
}

fn require_exact_registry(
    component_topology: &ComponentTopology,
    expected: &FleetRegistry,
    live: &LiveRegistryEvidence,
    stage: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = FleetRegistryOps::manifest(&expected.authority, component_topology, expected)?;
    let version = FleetRegistryOps::version(&expected.authority, component_topology, expected)?;
    let expected_evidence = RegistryEvidence {
        registry: expected,
        manifest: &manifest,
        version: &version,
    };
    if RegistryEvidence::from_live(live) != expected_evidence {
        return Err(FleetRegistryActivationError::LiveRegistryMismatch(stage).into());
    }
    Ok(())
}

fn require_exact_or_service_successor_registry(
    component_topology: &ComponentTopology,
    expected: &FleetRegistry,
    live: &LiveRegistryEvidence,
    expected_operation_id: [u8; 32],
    expected_plan_hash: [u8; 32],
    successor_evidence: Option<&ComponentProvisioningSuccessorEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_manifest =
        FleetRegistryOps::manifest(&expected.authority, component_topology, expected)?;
    let expected_version =
        FleetRegistryOps::version(&expected.authority, component_topology, expected)?;
    let expected_evidence = RegistryEvidence {
        registry: expected,
        manifest: &expected_manifest,
        version: &expected_version,
    };
    if RegistryEvidence::from_live(live) == expected_evidence {
        return Ok(());
    }

    FleetRegistryOps::validate(&expected.authority, component_topology, &live.registry)?;
    let live_manifest =
        FleetRegistryOps::manifest(&live.registry.authority, component_topology, &live.registry)?;
    let live_version =
        FleetRegistryOps::version(&live.registry.authority, component_topology, &live.registry)?;
    let expected_successor_revision = expected.revision.checked_add(1);
    let immutable_authority_matches = RegistryImmutableAuthority::from_registry(&live.registry)
        == RegistryImmutableAuthority::from_registry(expected);
    let service_successor_facts = [
        expected_successor_revision == Some(live.registry.revision),
        expected.services.is_empty(),
        !live.registry.services.is_empty(),
    ];
    let is_service_successor = service_successor_facts
        .into_iter()
        .all(std::convert::identity);
    let evidence_is_exact = RegistryHead {
        manifest: &live.manifest,
        version: &live.version,
    } == RegistryHead {
        manifest: &live_manifest,
        version: &live_version,
    };
    let successor_is_valid = [
        immutable_authority_matches,
        is_service_successor,
        evidence_is_exact,
        successor_evidence.is_some_and(|evidence| {
            evidence.operation_id == expected_operation_id
                && evidence.plan_hash == expected_plan_hash
                && evidence.source_registry == expected_version
                && evidence.published_registry.as_ref() == Some(&live.version)
                && evidence.operation == FleetComponentProvisioningOperation::FreshInstall
                && component_provisioning_phase_has_published_services(evidence.phase)
        }),
    ]
    .into_iter()
    .all(std::convert::identity);
    if !successor_is_valid {
        return Err(FleetRegistryActivationError::LiveRegistryMismatch(
            "verified all-Active or exact Fleet-service successor",
        )
        .into());
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentProvisioningSuccessorEvidence {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    source_registry: FleetRegistryVersion,
    published_registry: Option<FleetRegistryVersion>,
    operation: FleetComponentProvisioningOperation,
    phase: FleetComponentProvisioningPhase,
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

fn query_component_provisioning_successor_evidence(
    icp: &IcpCli,
    binding: &crate::protocol_binding::ResolvedProtocolBinding,
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
        return Err(FleetRegistryActivationError::LiveRegistryMismatch(
            "service-successor provisioning evidence",
        )
        .into());
    };
    Ok(ComponentProvisioningSuccessorEvidence::from(&status))
}

#[derive(Eq, PartialEq)]
struct RegistryEvidence<'a> {
    registry: &'a FleetRegistry,
    manifest: &'a FleetRegistryManifest,
    version: &'a FleetRegistryVersion,
}

#[derive(Eq, PartialEq)]
struct AcknowledgementAuthority<'a> {
    root: Principal,
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
    component_specs: &'a [FleetComponentSpecEntry],
    fleet_subnet_roots: &'a [FleetSubnetRootEntry],
}

impl<'a> RegistryImmutableAuthority<'a> {
    fn from_registry(registry: &'a FleetRegistry) -> Self {
        Self {
            authority: &registry.authority,
            component_specs: &registry.component_specs,
            fleet_subnet_roots: &registry.fleet_subnet_roots,
        }
    }
}
