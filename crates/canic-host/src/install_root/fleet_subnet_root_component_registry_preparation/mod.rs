//! Module: install_root::fleet_subnet_root_component_registry_preparation
//!
//! Responsibility: prepare and independently verify every root's empty Component Registry.
//! Does not own: Component allocation, paid Canister effects, or runtime activation.
//! Boundary: each root journal freezes exact Store and active Registry authority before mutation.

use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_component_registry_preparation, plan_fleet_subnet_root_install,
    record_component_registry_preparation_verified, record_component_registry_prepared,
};
use super::icp_context::InstallIcpContext;
use super::operations::call_with_arg;
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan,
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_set::{AppConfigSnapshot, load_persisted_canic_infrastructure_artifact_manifest},
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::component_registry::{
        RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
    },
    protocol,
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error as ThisError;

const MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS: usize = 4;

#[derive(CandidType)]
enum RootCommandFragment {
    PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    PrepareComponentRegistry(RootComponentRegistryStatusResponse),
}

#[derive(Debug, ThisError)]
enum RootComponentRegistryPreparationError {
    #[error("root Component Registry preparation reached unexpected phase {0:?}")]
    UnexpectedPhase(FleetSubnetRootInstallPhase),

    #[error("root Component Registry preparation exceeded its bounded phase transitions")]
    TransitionBoundExceeded,

    #[error("live root Component Registry differs from durable preparation evidence")]
    LiveEvidenceMismatch,

    #[error("live root Component Registry progress regressed from durable preparation evidence")]
    LiveProgressRegressed,
}

#[derive(Eq, PartialEq)]
struct ComponentRegistryAuthority<'a> {
    fleet_subnet_root: Principal,
    prepared_against_registry: &'a canic_core::dto::fleet_registry::FleetRegistryVersion,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    component_topology_digest: canic_core::ids::ComponentTopologyDigest,
}

impl<'a> From<&'a RootComponentRegistryStatusResponse> for ComponentRegistryAuthority<'a> {
    fn from(status: &'a RootComponentRegistryStatusResponse) -> Self {
        Self {
            fleet_subnet_root: status.fleet_subnet_root,
            prepared_against_registry: &status.prepared_against_registry,
            release_set: status.release_set,
            component_topology_digest: status.component_topology_digest,
        }
    }
}

/// Read the exact retained Component Registry authority through its status-like replay path.
///
/// Once preparation exists, the protected Root-owned command verifies the durable Store bootstrap,
/// active Registry mirror and immutable Component Registry preparation authority, then returns the
/// live status without mutation. The host binds those exact fields to the retained empty-registry
/// receipt while allowing allocation counters to advance. Matching controllers, authority and
/// module bytes alone cannot establish that property.
pub(super) fn verify_retained_component_registry_preparation(
    icp_context: &InstallIcpContext,
    journal: &super::fleet_subnet_root_install_journal::FleetSubnetRootInstallJournal,
) -> Result<(), Box<dyn std::error::Error>> {
    if journal.phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified {
        return Err(RootComponentRegistryPreparationError::UnexpectedPhase(journal.phase).into());
    }
    let root = journal
        .fleet_subnet_root
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let request = journal
        .component_registry_preparation_request
        .clone()
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let expected = journal
        .component_registry_preparation_response
        .as_ref()
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &journal.root_artifact,
    )?;
    let response: RootCommandResponseFragment = call_with_arg(
        icp_context.cli(),
        &binding,
        root,
        protocol::CANIC_COMMAND,
        &RootCommandFragment::PrepareComponentRegistry(request),
    )?;
    let RootCommandResponseFragment::PrepareComponentRegistry(observed) = response;
    validate_retained_component_registry_progress(expected, &observed)
}

fn validate_retained_component_registry_progress(
    retained: &RootComponentRegistryStatusResponse,
    observed: &RootComponentRegistryStatusResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    let authority_matches =
        ComponentRegistryAuthority::from(retained) == ComponentRegistryAuthority::from(observed);
    if !authority_matches {
        return Err(RootComponentRegistryPreparationError::LiveEvidenceMismatch.into());
    }
    let retained_occupied = retained
        .reserved_component_instances
        .checked_add(retained.committed_component_instances)
        .ok_or(RootComponentRegistryPreparationError::LiveProgressRegressed)?;
    let observed_occupied = observed
        .reserved_component_instances
        .checked_add(observed.committed_component_instances)
        .ok_or(RootComponentRegistryPreparationError::LiveProgressRegressed)?;
    let progress_is_monotonic = observed.next_allocation_sequence
        >= retained.next_allocation_sequence
        && observed_occupied >= retained_occupied
        && observed.committed_component_instances >= retained.committed_component_instances
        && observed.managed_descendants >= retained.managed_descendants
        && observed.known_created_component_canisters >= retained.known_created_component_canisters
        && observed.encoded_bytes >= retained.encoded_bytes;
    if !progress_is_monotonic {
        return Err(RootComponentRegistryPreparationError::LiveProgressRegressed.into());
    }
    if let Some(retained_inventory) = retained.initial_inventory {
        let Some(observed_inventory) = observed.initial_inventory else {
            return Err(RootComponentRegistryPreparationError::LiveProgressRegressed.into());
        };
        let same_inventory_authority = retained_inventory.fleet_activation_operation_id
            == observed_inventory.fleet_activation_operation_id
            && retained_inventory.component_count == observed_inventory.component_count
            && retained_inventory.inventory_hash == observed_inventory.inventory_hash
            && retained_inventory.sealed_at_ns == observed_inventory.sealed_at_ns;
        let inventory_progress_is_monotonic = (!retained_inventory.directories_converged
            || observed_inventory.directories_converged)
            && (!retained_inventory.root_runtime_activated
                || observed_inventory.root_runtime_activated);
        if !same_inventory_authority || !inventory_progress_is_monotonic {
            return Err(RootComponentRegistryPreparationError::LiveProgressRegressed.into());
        }
    }
    Ok(())
}

pub(super) struct PrepareFleetSubnetRootComponentRegistriesRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub config_path: &'a Path,
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
}

pub(super) fn prepare_and_verify_fleet_subnet_root_component_registries(
    request: PrepareFleetSubnetRootComponentRegistriesRequest<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(request.config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        request.icp.root(),
        request.fleet_install_plan.plan.release_build_id,
    )?;

    for root_plan in &request.fleet_install_plan.plan.fleet_subnet_roots {
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: request.fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator: request.coordinator,
            install_operation_id: request.install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        let mirror_request = current
            .journal
            .registry_mirror_activation_request
            .clone()
            .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
        let preparation_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: mirror_request.store_bootstrap,
            expected_fleet_registry: mirror_request.expected_registry,
        };
        drive_component_registry_preparation(request.icp, current, preparation_request)?;
    }
    Ok(())
}

fn drive_component_registry_preparation(
    icp_context: &InstallIcpContext,
    mut current: ResolvedFleetSubnetRootInstall,
    request: RootComponentRegistryPreparationRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .ok_or(RootComponentRegistryPreparationError::LiveEvidenceMismatch)?;
    let binding = resolve_infrastructure_protocol_binding(
        icp_context.root(),
        icp_context.environment(),
        &current.journal.root_artifact,
    )?;
    let icp = icp_context.cli();
    for _ in 0..MAX_COMPONENT_REGISTRY_PREPARATION_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => {
                begin_component_registry_preparation(&current, request.clone())?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => {
                let response: RootCommandResponseFragment = call_with_arg(
                    icp,
                    &binding,
                    root,
                    protocol::CANIC_COMMAND,
                    &RootCommandFragment::PrepareComponentRegistry(request.clone()),
                )?;
                let RootCommandResponseFragment::PrepareComponentRegistry(response) = response;
                record_component_registry_prepared(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPrepared => {
                let response: RootCommandResponseFragment = call_with_arg(
                    icp,
                    &binding,
                    root,
                    protocol::CANIC_COMMAND,
                    &RootCommandFragment::PrepareComponentRegistry(request.clone()),
                )?;
                let RootCommandResponseFragment::PrepareComponentRegistry(response) = response;
                record_component_registry_preparation_verified(&current, response)?
            }
            FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => return Ok(()),
            phase => {
                return Err(RootComponentRegistryPreparationError::UnexpectedPhase(phase).into());
            }
        };
    }
    Err(RootComponentRegistryPreparationError::TransitionBoundExceeded.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        dto::fleet_registry::FleetRegistryVersion,
        ids::{
            AppId, CanonicalNetworkId, ComponentTopologyDigest, FleetCoordinatorBinding, FleetId,
            FleetKey, FleetRegistryAuthority, FleetSubnetRootReleaseSet, ReleaseBuildId,
            ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn retained_component_registry_proof_accepts_monotonic_allocation_progress() {
        let retained = status_fixture();
        let observed = RootComponentRegistryStatusResponse {
            next_allocation_sequence: 2,
            reserved_component_instances: 1,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            encoded_bytes: 1_024,
            ..retained.clone()
        };

        validate_retained_component_registry_progress(&retained, &observed)
            .expect("monotonic allocation progress must remain valid repair evidence");
    }

    #[test]
    fn retained_component_registry_proof_accepts_reservation_commitment() {
        let retained = RootComponentRegistryStatusResponse {
            next_allocation_sequence: 2,
            reserved_component_instances: 1,
            encoded_bytes: 1_024,
            ..status_fixture()
        };
        let observed = RootComponentRegistryStatusResponse {
            reserved_component_instances: 0,
            committed_component_instances: 1,
            encoded_bytes: 2_048,
            ..retained.clone()
        };

        validate_retained_component_registry_progress(&retained, &observed)
            .expect("committing one retained reservation is monotonic Registry progress");
    }

    #[test]
    fn retained_component_registry_proof_rejects_authority_drift_and_regression() {
        let retained = RootComponentRegistryStatusResponse {
            next_allocation_sequence: 2,
            reserved_component_instances: 1,
            encoded_bytes: 1_024,
            ..status_fixture()
        };

        let mut changed_root = retained.clone();
        changed_root.fleet_subnet_root = Principal::from_slice(&[9; 29]);
        assert!(validate_retained_component_registry_progress(&retained, &changed_root).is_err());

        let mut changed_registry = retained.clone();
        changed_registry.prepared_against_registry.content_hash = [10; 32];
        assert!(
            validate_retained_component_registry_progress(&retained, &changed_registry).is_err()
        );

        let mut changed_release = retained.clone();
        changed_release.release_set.manifest_digest = ReleaseSetDigest::from_bytes([11; 32]);
        assert!(
            validate_retained_component_registry_progress(&retained, &changed_release).is_err()
        );

        let mut changed_topology = retained.clone();
        changed_topology.component_topology_digest = ComponentTopologyDigest::from_bytes([12; 32]);
        assert!(
            validate_retained_component_registry_progress(&retained, &changed_topology).is_err()
        );

        let mut regressed = retained.clone();
        regressed.reserved_component_instances = 0;
        assert!(validate_retained_component_registry_progress(&retained, &regressed).is_err());
    }

    fn status_fixture() -> RootComponentRegistryStatusResponse {
        RootComponentRegistryStatusResponse {
            fleet_subnet_root: Principal::from_slice(&[4; 29]),
            prepared_against_registry: FleetRegistryVersion {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: canic_core::ids::FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                fleet_id: FleetId::from_generated_bytes([1; 32]),
                            },
                            app: AppId::from("repair-proof"),
                        },
                        coordinator_subnet: SubnetId::from_principal(Principal::from_slice(
                            &[2; 29],
                        )),
                        coordinator: Principal::from_slice(&[3; 29]),
                    },
                    epoch: 1,
                },
                revision: 2,
                content_hash: [5; 32],
            },
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [6; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([7; 32]),
            },
            component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            encoded_bytes: 0,
            initial_inventory: None,
        }
    }
}
