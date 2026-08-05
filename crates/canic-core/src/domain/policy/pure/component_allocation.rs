//! Module: domain::policy::pure::component_allocation
//!
//! Responsibility: decide whether one top-level Component identity may be reserved.
//! Does not own: storage, authentication, Store resolution, or Canister effects.
//! Boundary: workflow supplies exact root authority, topology, and durable allocation counts.

use crate::{
    config::{ComponentProvisioningGrant, ComponentTopology},
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecAdmission,
        ComponentSpecId, FleetSubnetRootBinding,
    },
};
use thiserror::Error as ThisError;

///
/// TopLevelComponentAllocationInput
///
/// Complete pure evidence needed to reserve one top-level Component identity.
///

pub struct TopLevelComponentAllocationInput<'a> {
    pub operation_id: [u8; 32],
    pub component_spec: &'a ComponentSpecId,
    pub root: &'a FleetSubnetRootBinding,
    pub topology: &'a ComponentTopology,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub reserved_spec_instances: u32,
    pub committed_spec_instances: u32,
}

///
/// TopLevelComponentAllocationDecision
///
/// Deterministic identity and admitted Spec facts approved for durable reservation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopLevelComponentAllocationDecision {
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
}

///
/// PeerComponentProvisioningReadiness
///
/// Runtime state supplied to the pure same-root peer-provisioning decision.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerComponentProvisioningReadiness {
    RootRuntimeInactive,
    RequesterRegistryMemberInactive,
    Ready,
}

///
/// PeerComponentProvisioningInput
///
/// Complete pure authority and capacity evidence for one peer Component request.
///

pub struct PeerComponentProvisioningInput<'a> {
    pub requester: &'a ComponentBinding,
    pub target_component_spec: &'a ComponentSpecId,
    pub root: &'a FleetSubnetRootBinding,
    pub topology: &'a ComponentTopology,
    pub readiness: PeerComponentProvisioningReadiness,
    pub reserved_peer_instances: u32,
    pub committed_peer_instances: u32,
}

///
/// ComponentAllocationPolicyError
///
/// Typed rejection before any root-local allocation mutation.
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum ComponentAllocationPolicyError {
    #[error("Component allocation operation ID must not be all zeroes")]
    EmptyOperationId,

    #[error("Component allocation sequence is exhausted")]
    AllocationSequenceExhausted,

    #[error("Component topology cannot be projected for the protected root admissions")]
    InvalidRootTopologyProjection,

    #[error("protected root admissions do not reproduce its Component Topology digest")]
    RootTopologyDigestMismatch,

    #[error("Component Spec '{0}' is not admitted on this Fleet Subnet Root")]
    ComponentSpecNotAdmitted(ComponentSpecId),

    #[error("Component Spec '{0}' does not exist in the protected root topology")]
    ComponentSpecUnknown(ComponentSpecId),

    #[error("Component Spec '{0}' admission hash differs from its protected topology")]
    ComponentSpecHashMismatch(ComponentSpecId),

    #[error("root Component instance count overflowed")]
    ComponentCountOverflow,

    #[error("root Component instance capacity is exhausted")]
    ComponentCapacityExhausted,

    #[error("Component Spec '{0}' root-local instance count overflowed")]
    ComponentSpecCountOverflow(ComponentSpecId),

    #[error("Component Spec '{0}' root-local admission is exhausted")]
    ComponentSpecCapacityExhausted(ComponentSpecId),

    #[error("peer Component provisioning has invalid requester binding authority")]
    InvalidPeerRequesterBinding,

    #[error("peer Component provisioning requires an Active Fleet Subnet Root runtime")]
    PeerRootRuntimeInactive,

    #[error("peer Component provisioning requires an Active requester Registry member")]
    PeerRequesterRegistryMemberInactive,

    #[error(
        "Component Spec '{requester_component_spec}' has no provisioning grant for peer Spec '{target_component_spec}'"
    )]
    PeerProvisioningGrantMissing {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },

    #[error("peer Component provisioning count overflowed")]
    PeerProvisioningCountOverflow,

    #[error(
        "Component Spec '{requester_component_spec}' exhausted its per-requester/root provisioning capacity for peer Spec '{target_component_spec}'"
    )]
    PeerProvisioningCapacityExhausted {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },
}

/// Authorize one exact same-root peer Component request without creating parentage.
pub fn authorize_peer_component_provisioning(
    input: PeerComponentProvisioningInput<'_>,
) -> Result<ComponentProvisioningGrant, ComponentAllocationPolicyError> {
    input
        .topology
        .validate_component_binding(input.root, input.requester)
        .map_err(|_| ComponentAllocationPolicyError::InvalidPeerRequesterBinding)?;
    match input.readiness {
        PeerComponentProvisioningReadiness::RootRuntimeInactive => {
            return Err(ComponentAllocationPolicyError::PeerRootRuntimeInactive);
        }
        PeerComponentProvisioningReadiness::RequesterRegistryMemberInactive => {
            return Err(ComponentAllocationPolicyError::PeerRequesterRegistryMemberInactive);
        }
        PeerComponentProvisioningReadiness::Ready => {}
    }

    let grant = input
        .topology
        .provisioning_grant(&input.requester.component_spec, input.target_component_spec)
        .ok_or_else(
            || ComponentAllocationPolicyError::PeerProvisioningGrantMissing {
                requester_component_spec: input.requester.component_spec.clone(),
                target_component_spec: input.target_component_spec.clone(),
            },
        )?;
    let allocated = input
        .reserved_peer_instances
        .checked_add(input.committed_peer_instances)
        .ok_or(ComponentAllocationPolicyError::PeerProvisioningCountOverflow)?;
    if allocated >= grant.maximum_instances_per_requester_per_root {
        return Err(
            ComponentAllocationPolicyError::PeerProvisioningCapacityExhausted {
                requester_component_spec: input.requester.component_spec.clone(),
                target_component_spec: input.target_component_spec.clone(),
            },
        );
    }

    Ok(grant.clone())
}

/// Decide one exact top-level Component identity reservation without mutation.
pub fn reserve_top_level_component(
    input: TopLevelComponentAllocationInput<'_>,
) -> Result<TopLevelComponentAllocationDecision, ComponentAllocationPolicyError> {
    if input.operation_id == [0; 32] {
        return Err(ComponentAllocationPolicyError::EmptyOperationId);
    }
    input
        .next_allocation_sequence
        .checked_add(1)
        .ok_or(ComponentAllocationPolicyError::AllocationSequenceExhausted)?;

    let projected = input
        .topology
        .project_for_admissions(&input.root.component_admissions)
        .map_err(|_| ComponentAllocationPolicyError::InvalidRootTopologyProjection)?;
    let projected_digest = projected
        .digest()
        .map_err(|_| ComponentAllocationPolicyError::InvalidRootTopologyProjection)?;
    if projected_digest != input.root.component_topology_digest {
        return Err(ComponentAllocationPolicyError::RootTopologyDigestMismatch);
    }

    let admission = admission(input.root, input.component_spec)?;
    let spec = projected.get(input.component_spec).ok_or_else(|| {
        ComponentAllocationPolicyError::ComponentSpecUnknown(input.component_spec.clone())
    })?;
    if admission.spec_hash != spec.spec_hash {
        return Err(ComponentAllocationPolicyError::ComponentSpecHashMismatch(
            input.component_spec.clone(),
        ));
    }

    let allocated_components = input
        .reserved_component_instances
        .checked_add(input.committed_component_instances)
        .ok_or(ComponentAllocationPolicyError::ComponentCountOverflow)?;
    if allocated_components >= input.root.limits.maximum_component_instances {
        return Err(ComponentAllocationPolicyError::ComponentCapacityExhausted);
    }

    let spec_instances = input
        .reserved_spec_instances
        .checked_add(input.committed_spec_instances)
        .ok_or_else(|| {
            ComponentAllocationPolicyError::ComponentSpecCountOverflow(input.component_spec.clone())
        })?;
    if spec_instances >= admission.maximum_root_instances {
        return Err(
            ComponentAllocationPolicyError::ComponentSpecCapacityExhausted(
                input.component_spec.clone(),
            ),
        );
    }

    let allocation_sequence = input.next_allocation_sequence;
    Ok(TopLevelComponentAllocationDecision {
        allocation_sequence,
        component: ComponentInstanceId::from_root_allocation(
            input.root.authority.binding.fleet.fleet,
            input.root.authority.epoch,
            input.root.fleet_subnet_root,
            allocation_sequence,
        ),
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        role: spec.component_role.clone(),
    })
}

fn admission<'a>(
    root: &'a FleetSubnetRootBinding,
    component_spec: &ComponentSpecId,
) -> Result<&'a ComponentSpecAdmission, ComponentAllocationPolicyError> {
    root.component_admissions
        .binary_search_by(|candidate| candidate.component_spec.cmp(component_spec))
        .ok()
        .map(|index| &root.component_admissions[index])
        .ok_or_else(|| {
            ComponentAllocationPolicyError::ComponentSpecNotAdmitted(component_spec.clone())
        })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        config::{
            ComponentTopology,
            schema::{ConfigModel, Validate},
        },
        ids::{
            AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetRootLimits, SubnetId,
        },
    };

    const CONFIG: &str = r#"
        [app]
        name = "toko"

        [roles.root]
        package = "root"
        kind = "root"

        [roles.project_hub]
        package = "project_hub"
        kind = "canister"

        [component_specs.projects]
        component_role = "project_hub"
        maximum_instances = 4
    "#;

    const PEER_CONFIG: &str = r#"
        [app]
        name = "toko"

        [roles.root]
        package = "root"
        kind = "root"

        [roles.project_hub]
        package = "project_hub"
        kind = "canister"

        [roles.user_hub]
        package = "user_hub"
        kind = "canister"

        [component_specs.projects]
        component_role = "project_hub"
        maximum_instances = 4

        [component_specs.projects.provisions.users]
        maximum_instances_per_requester_per_root = 2

        [component_specs.users]
        component_role = "user_hub"
        maximum_instances = 4
    "#;

    #[test]
    fn reservation_derives_exact_root_local_identity() {
        let (topology, root, component_spec) = fixture();
        let decision = reserve_top_level_component(input(&topology, &root, &component_spec))
            .expect("reservation decision");

        assert_eq!(decision.allocation_sequence, 1);
        assert_eq!(decision.component_spec, component_spec);
        assert_eq!(decision.spec_hash, root.component_admissions[0].spec_hash);
        assert_eq!(decision.role, CanisterRole::new("project_hub"));
        assert_eq!(
            decision.component,
            ComponentInstanceId::from_root_allocation(
                root.authority.binding.fleet.fleet,
                root.authority.epoch,
                root.fleet_subnet_root,
                1,
            )
        );
    }

    #[test]
    fn reservation_rejects_empty_operation_and_exhausted_sequence() {
        let (topology, root, component_spec) = fixture();
        let mut request = input(&topology, &root, &component_spec);
        request.operation_id = [0; 32];
        assert_eq!(
            reserve_top_level_component(request),
            Err(ComponentAllocationPolicyError::EmptyOperationId)
        );

        let mut request = input(&topology, &root, &component_spec);
        request.next_allocation_sequence = u64::MAX;
        assert_eq!(
            reserve_top_level_component(request),
            Err(ComponentAllocationPolicyError::AllocationSequenceExhausted)
        );
    }

    #[test]
    fn reservation_enforces_spec_and_component_capacity() {
        let (topology, mut root, component_spec) = fixture();
        let mut request = input(&topology, &root, &component_spec);
        request.reserved_spec_instances = 2;
        assert_eq!(
            reserve_top_level_component(request),
            Err(
                ComponentAllocationPolicyError::ComponentSpecCapacityExhausted(
                    component_spec.clone()
                )
            )
        );

        root.limits.maximum_component_instances = 1;
        let mut request = input(&topology, &root, &component_spec);
        request.committed_component_instances = 1;
        assert_eq!(
            reserve_top_level_component(request),
            Err(ComponentAllocationPolicyError::ComponentCapacityExhausted)
        );
    }

    #[test]
    fn reservation_rejects_unknown_and_hash_drifted_specs() {
        let (topology, mut root, component_spec) = fixture();
        let unknown = "users".parse().expect("Component Spec ID");
        assert_eq!(
            reserve_top_level_component(input(&topology, &root, &unknown)),
            Err(ComponentAllocationPolicyError::ComponentSpecNotAdmitted(
                unknown
            ))
        );

        let topology_digest = root.component_topology_digest;
        root.component_topology_digest = crate::ids::ComponentTopologyDigest::from_bytes([98; 32]);
        assert_eq!(
            reserve_top_level_component(input(&topology, &root, &component_spec)),
            Err(ComponentAllocationPolicyError::RootTopologyDigestMismatch)
        );
        root.component_topology_digest = topology_digest;

        root.component_admissions[0].spec_hash = [99; 32];
        assert_eq!(
            reserve_top_level_component(input(&topology, &root, &component_spec)),
            Err(ComponentAllocationPolicyError::InvalidRootTopologyProjection)
        );
    }

    #[test]
    fn peer_provisioning_returns_the_exact_non_parent_grant() {
        let (topology, root, requester, target) = peer_fixture();

        let grant = authorize_peer_component_provisioning(peer_input(
            &topology, &root, &requester, &target,
        ))
        .expect("peer provisioning decision");

        assert_eq!(grant.requester_component_spec, requester.component_spec);
        assert_eq!(grant.target_component_spec, target);
        assert_eq!(grant.maximum_instances_per_requester_per_root, 2);
    }

    #[test]
    fn peer_provisioning_rejects_missing_grant_inactive_requester_and_exhausted_ceiling() {
        let (topology, root, requester, target) = peer_fixture();
        let mut request = peer_input(&topology, &root, &requester, &target);
        request.readiness = PeerComponentProvisioningReadiness::RequesterRegistryMemberInactive;
        assert_eq!(
            authorize_peer_component_provisioning(request),
            Err(ComponentAllocationPolicyError::PeerRequesterRegistryMemberInactive)
        );

        let mut request = peer_input(&topology, &root, &requester, &target);
        request.committed_peer_instances = 2;
        assert_eq!(
            authorize_peer_component_provisioning(request),
            Err(
                ComponentAllocationPolicyError::PeerProvisioningCapacityExhausted {
                    requester_component_spec: requester.component_spec.clone(),
                    target_component_spec: target.clone(),
                }
            )
        );

        let missing_target = requester.component_spec.clone();
        assert_eq!(
            authorize_peer_component_provisioning(peer_input(
                &topology,
                &root,
                &requester,
                &missing_target,
            )),
            Err(
                ComponentAllocationPolicyError::PeerProvisioningGrantMissing {
                    requester_component_spec: requester.component_spec.clone(),
                    target_component_spec: missing_target,
                }
            )
        );
    }

    fn fixture() -> (ComponentTopology, FleetSubnetRootBinding, ComponentSpecId) {
        let config: ConfigModel = toml::from_str(CONFIG).expect("config");
        config.validate().expect("valid config");
        let topology = ComponentTopology::compile(&config).expect("topology");
        let component_spec = "projects".parse().expect("Component Spec ID");
        let spec = topology.get(&component_spec).expect("projects Spec");
        let admissions = vec![ComponentSpecAdmission {
            component_spec: component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: 2,
        }];
        let projected = topology
            .project_for_admissions(&admissions)
            .expect("root topology projection");
        let root = FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                        &[2; 29],
                    )),
                    coordinator: candid::Principal::from_slice(&[3; 29]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
            component_admissions: admissions,
            component_topology_digest: projected.digest().expect("root topology digest"),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 4,
                maximum_registry_bytes: 1_000_000,
                maximum_wasm_store_bytes: 1_000_000,
                canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(5_000_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
        };
        (topology, root, component_spec)
    }

    fn input<'a>(
        topology: &'a ComponentTopology,
        root: &'a FleetSubnetRootBinding,
        component_spec: &'a ComponentSpecId,
    ) -> TopLevelComponentAllocationInput<'a> {
        TopLevelComponentAllocationInput {
            operation_id: [7; 32],
            component_spec,
            root,
            topology,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            reserved_spec_instances: 0,
            committed_spec_instances: 0,
        }
    }

    fn peer_fixture() -> (
        ComponentTopology,
        FleetSubnetRootBinding,
        ComponentBinding,
        ComponentSpecId,
    ) {
        let config: ConfigModel = toml::from_str(PEER_CONFIG).expect("peer config");
        config.validate().expect("valid peer config");
        let topology = ComponentTopology::compile(&config).expect("peer topology");
        let requester_spec = "projects".parse().expect("requester Component Spec ID");
        let target_spec = "users".parse().expect("target Component Spec ID");
        let admissions = topology
            .component_specs
            .iter()
            .map(|spec| ComponentSpecAdmission {
                component_spec: spec.component_spec.clone(),
                spec_hash: spec.spec_hash,
                maximum_root_instances: 4,
            })
            .collect::<Vec<_>>();
        let projected = topology
            .project_for_admissions(&admissions)
            .expect("peer root topology projection");
        let root = FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([11; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                        &[12; 29],
                    )),
                    coordinator: candid::Principal::from_slice(&[13; 29]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[14; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[15; 29]),
            component_admissions: admissions,
            component_topology_digest: projected.digest().expect("peer root topology digest"),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 8,
                maximum_registry_bytes: 1_000_000,
                maximum_wasm_store_bytes: 1_000_000,
                canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(5_000_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
        };
        let requester_compiled = topology.get(&requester_spec).expect("requester Spec");
        let requester = ComponentBinding {
            authority: root.authority.clone(),
            component: ComponentInstanceId::from_root_allocation(
                root.authority.binding.fleet.fleet,
                root.authority.epoch,
                root.fleet_subnet_root,
                1,
            ),
            component_spec: requester_spec,
            spec_hash: requester_compiled.spec_hash,
            role: requester_compiled.component_role.clone(),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: candid::Principal::from_slice(&[16; 29]),
        };
        (topology, root, requester, target_spec)
    }

    const fn peer_input<'a>(
        topology: &'a ComponentTopology,
        root: &'a FleetSubnetRootBinding,
        requester: &'a ComponentBinding,
        target: &'a ComponentSpecId,
    ) -> PeerComponentProvisioningInput<'a> {
        PeerComponentProvisioningInput {
            requester,
            target_component_spec: target,
            root,
            topology,
            readiness: PeerComponentProvisioningReadiness::Ready,
            reserved_peer_instances: 0,
            committed_peer_instances: 0,
        }
    }
}
