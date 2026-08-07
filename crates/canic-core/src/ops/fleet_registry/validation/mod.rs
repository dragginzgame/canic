//! Module: ops::fleet_registry::validation
//!
//! Responsibility: validate passive Fleet Registry snapshots and compile genesis.
//! Does not own: stable storage, snapshot publication, transport, or root lifecycle effects.
//! Boundary: the parent ops module calls this against one compiled Component Topology.

use crate::{
    config::{ComponentTopology, ComponentTopologyError, FleetServiceMemberPurpose},
    dto::fleet_registry::{
        FleetComponentSpecEntry, FleetRegistry, FleetServiceBinding, FleetServiceComponentBinding,
        FleetServiceMode, FleetSubnetRootEntry, FleetSubnetRootStatus,
    },
    ids::{AppId, ComponentInstanceId, FleetRegistryAuthority, FleetServiceId},
    ops::fleet_registry::FleetRegistryOpsError,
};
use std::collections::{BTreeMap, BTreeSet};

use candid::Principal;

pub(super) fn compile_genesis(
    configured_app: &AppId,
    authority: FleetRegistryAuthority,
    topology: &ComponentTopology,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    if authority.epoch != 1 {
        return Err(FleetRegistryOpsError::GenesisAuthorityEpoch(
            authority.epoch,
        ));
    }
    if &authority.binding.fleet.app != configured_app {
        return Err(FleetRegistryOpsError::GenesisAppMismatch {
            expected: configured_app.clone(),
            received: authority.binding.fleet.app,
        });
    }

    let registry = FleetRegistry {
        authority: authority.clone(),
        revision: 1,
        component_specs: topology
            .component_specs
            .iter()
            .map(|spec| FleetComponentSpecEntry {
                component_spec: spec.component_spec.clone(),
                spec_hash: spec.spec_hash,
                component_role: spec.component_role.clone(),
                maximum_fleet_instances: spec.maximum_fleet_instances,
            })
            .collect(),
        fleet_subnet_roots: Vec::new(),
        services: Vec::new(),
    };
    validate(&authority, topology, &registry)?;
    Ok(registry)
}

pub(super) fn validate(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryOpsError> {
    validate_authority(&registry.authority)?;
    if &registry.authority != expected_authority {
        return Err(FleetRegistryOpsError::AuthorityMismatch);
    }
    if registry.revision == 0 {
        return Err(FleetRegistryOpsError::NonPositiveRevision);
    }

    validate_component_specs(topology, &registry.component_specs)?;
    validate_roots(topology, registry)?;
    validate_services(registry)
}

fn validate_authority(authority: &FleetRegistryAuthority) -> Result<(), FleetRegistryOpsError> {
    if authority.epoch == 0 {
        return Err(FleetRegistryOpsError::NonPositiveAuthorityEpoch);
    }
    if authority.binding.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousCoordinatorSubnet);
    }
    if authority.binding.coordinator == Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousCoordinator);
    }
    Ok(())
}

fn validate_component_specs(
    topology: &ComponentTopology,
    entries: &[FleetComponentSpecEntry],
) -> Result<(), FleetRegistryOpsError> {
    if entries.len() != topology.component_specs.len() {
        return Err(FleetRegistryOpsError::FleetComponentSpecSetMismatch);
    }

    for (entry, expected) in entries.iter().zip(&topology.component_specs) {
        if entry.component_spec != expected.component_spec
            || entry.spec_hash != expected.spec_hash
            || entry.component_role != expected.component_role
            || entry.maximum_fleet_instances != expected.maximum_fleet_instances
        {
            return Err(FleetRegistryOpsError::FleetComponentSpecMismatch {
                component_spec: entry.component_spec.clone(),
            });
        }
    }

    Ok(())
}

fn validate_roots(
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<(), FleetRegistryOpsError> {
    let mut previous_subnet = None;
    let mut release_build_id = None;
    let mut root_principals = BTreeSet::new();
    let mut admission_totals = topology
        .component_specs
        .iter()
        .map(|spec| (spec.component_spec.clone(), 0_u32))
        .collect::<BTreeMap<_, _>>();

    for root in &registry.fleet_subnet_roots {
        if previous_subnet.is_some_and(|previous| previous >= root.placement_subnet) {
            return Err(FleetRegistryOpsError::NonCanonicalFleetSubnetRootOrder);
        }
        previous_subnet = Some(root.placement_subnet);
        validate_root_identity(registry, root, &mut root_principals)?;
        topology.validate_planned_root(
            &root.component_admissions,
            root.component_topology_digest,
            &root.limits,
        )?;
        let root_release_build_id = root.active_release_set.release_build_id;
        if let Some(expected) = release_build_id {
            if root_release_build_id != expected {
                return Err(FleetRegistryOpsError::RootReleaseBuildMismatch {
                    expected,
                    received: root_release_build_id,
                });
            }
        } else {
            release_build_id = Some(root_release_build_id);
        }

        for admission in &root.component_admissions {
            let total = admission_totals
                .get_mut(&admission.component_spec)
                .expect("planned-root validation admitted only known Component Specs");
            *total = total
                .checked_add(admission.maximum_root_instances)
                .ok_or_else(|| FleetRegistryOpsError::FleetAdmissionsOverflow {
                    component_spec: admission.component_spec.clone(),
                })?;
        }
    }

    for spec in &topology.component_specs {
        let admitted = admission_totals[&spec.component_spec];
        if admitted > spec.maximum_fleet_instances {
            return Err(FleetRegistryOpsError::FleetAdmissionsExceedMaximum {
                component_spec: spec.component_spec.clone(),
                admitted,
                maximum_fleet_instances: spec.maximum_fleet_instances,
            });
        }
    }

    Ok(())
}

fn validate_root_identity(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    root_principals: &mut BTreeSet<Principal>,
) -> Result<(), FleetRegistryOpsError> {
    if root.placement_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetRegistryOpsError::Topology(
            ComponentTopologyError::AnonymousBindingPrincipal {
                field: "fleet_subnet_roots.placement_subnet",
            },
        ));
    }
    if root.fleet_subnet_root == Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousFleetSubnetRoot);
    }
    if root.fleet_subnet_root == registry.authority.binding.coordinator {
        return Err(FleetRegistryOpsError::RootPrincipalConflictsWithCoordinator);
    }
    if !root_principals.insert(root.fleet_subnet_root) {
        return Err(FleetRegistryOpsError::DuplicateFleetSubnetRoot {
            fleet_subnet_root: root.fleet_subnet_root,
        });
    }
    Ok(())
}

fn validate_services(registry: &FleetRegistry) -> Result<(), FleetRegistryOpsError> {
    let mut previous_service: Option<&FleetServiceId> = None;
    let mut components = BTreeSet::<ComponentInstanceId>::new();
    let mut canisters = BTreeSet::<Principal>::new();

    for service in &registry.services {
        if previous_service.is_some_and(|previous| previous >= &service.service) {
            return Err(FleetRegistryOpsError::NonCanonicalFleetServiceOrder);
        }
        previous_service = Some(&service.service);
        validate_service_spec(registry, service)?;
        validate_service_members(registry, service, &mut components, &mut canisters)?;
    }
    Ok(())
}

fn validate_service_spec(
    registry: &FleetRegistry,
    service: &FleetServiceBinding,
) -> Result<(), FleetRegistryOpsError> {
    let expected = FleetServiceSpecAuthority {
        component_spec: &service.component_spec,
        component_role: &service.role,
    };
    let matches_spec = registry.component_specs.iter().any(|spec| {
        FleetServiceSpecAuthority {
            component_spec: &spec.component_spec,
            component_role: &spec.component_role,
        } == expected
    });
    if !matches_spec {
        return Err(FleetRegistryOpsError::FleetServiceSpecMismatch {
            service: service.service.clone(),
        });
    }
    if service.members.is_empty() {
        return Err(FleetRegistryOpsError::EmptyFleetService {
            service: service.service.clone(),
        });
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct FleetServiceSpecAuthority<'a> {
    component_spec: &'a crate::ids::ComponentSpecId,
    component_role: &'a crate::ids::CanisterRole,
}

fn validate_service_members(
    registry: &FleetRegistry,
    service: &FleetServiceBinding,
    components: &mut BTreeSet<ComponentInstanceId>,
    canisters: &mut BTreeSet<Principal>,
) -> Result<(), FleetRegistryOpsError> {
    let mut previous: Option<&FleetServiceComponentBinding> = None;
    let mut root_counts = BTreeMap::<Principal, u32>::new();
    let mut purposes = FleetServicePurposeCounts::default();

    for member in &service.members {
        if previous.is_some_and(|one| compare_service_members(one, member).is_ge()) {
            return Err(FleetRegistryOpsError::NonCanonicalFleetServiceMemberOrder {
                service: service.service.clone(),
            });
        }
        previous = Some(member);
        validate_service_member_identity(member, components, canisters)?;
        validate_service_member_root(registry, service, member)?;
        let count = root_counts.entry(member.fleet_subnet_root).or_default();
        *count = count.checked_add(1).ok_or_else(|| {
            FleetRegistryOpsError::FleetServicePlacementMismatch {
                service: service.service.clone(),
            }
        })?;
        purposes.record(member.member_purpose).ok_or_else(|| {
            FleetRegistryOpsError::FleetServiceModeMismatch {
                service: service.service.clone(),
            }
        })?;
    }
    validate_service_mode(service, purposes)?;
    validate_service_placement(service, &root_counts)
}

fn validate_service_member_identity(
    member: &FleetServiceComponentBinding,
    components: &mut BTreeSet<ComponentInstanceId>,
    canisters: &mut BTreeSet<Principal>,
) -> Result<(), FleetRegistryOpsError> {
    if member.component.as_bytes() == &[0; 32] {
        return Err(FleetRegistryOpsError::EmptyFleetServiceComponentIdentity);
    }
    if member.canister_id == Principal::anonymous() {
        return Err(FleetRegistryOpsError::AnonymousFleetServiceComponent);
    }
    if !components.insert(member.component) {
        return Err(FleetRegistryOpsError::DuplicateFleetServiceComponent {
            component: member.component,
        });
    }
    if !canisters.insert(member.canister_id) {
        return Err(FleetRegistryOpsError::DuplicateFleetServiceCanister {
            canister_id: member.canister_id,
        });
    }
    Ok(())
}

fn validate_service_member_root(
    registry: &FleetRegistry,
    service: &FleetServiceBinding,
    member: &FleetServiceComponentBinding,
) -> Result<(), FleetRegistryOpsError> {
    let Some(root) = registry
        .fleet_subnet_roots
        .iter()
        .find(|root| root.fleet_subnet_root == member.fleet_subnet_root)
    else {
        return Err(FleetRegistryOpsError::FleetServiceRootMismatch {
            service: service.service.clone(),
        });
    };
    if root.status != FleetSubnetRootStatus::Active
        || !root_admits_service_spec(registry, root, service)
    {
        return Err(FleetRegistryOpsError::FleetServiceRootMismatch {
            service: service.service.clone(),
        });
    }
    Ok(())
}

fn root_admits_service_spec(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    service: &FleetServiceBinding,
) -> bool {
    let Some(spec) = registry
        .component_specs
        .iter()
        .find(|spec| spec.component_spec == service.component_spec)
    else {
        return false;
    };
    let expected = FleetServiceAdmissionAuthority {
        component_spec: &service.component_spec,
        spec_hash: spec.spec_hash,
    };
    root.component_admissions.iter().any(|admission| {
        FleetServiceAdmissionAuthority {
            component_spec: &admission.component_spec,
            spec_hash: admission.spec_hash,
        } == expected
    })
}

#[derive(Eq, PartialEq)]
struct FleetServiceAdmissionAuthority<'a> {
    component_spec: &'a crate::ids::ComponentSpecId,
    spec_hash: [u8; 32],
}

fn validate_service_mode(
    service: &FleetServiceBinding,
    purposes: FleetServicePurposeCounts,
) -> Result<(), FleetRegistryOpsError> {
    if !purposes.matches(service.mode) {
        return Err(FleetRegistryOpsError::FleetServiceModeMismatch {
            service: service.service.clone(),
        });
    }
    Ok(())
}

fn validate_service_placement(
    service: &FleetServiceBinding,
    root_counts: &BTreeMap<Principal, u32>,
) -> Result<(), FleetRegistryOpsError> {
    let policy = service.placement;
    let member_count = u32::try_from(service.members.len()).map_err(|_| {
        FleetRegistryOpsError::FleetServicePlacementMismatch {
            service: service.service.clone(),
        }
    })?;
    let required_roots = member_count.min(policy.minimum_distinct_roots) as usize;
    let assessment = FleetServicePlacementAssessment {
        maximum_members_per_root: policy.maximum_members_per_root,
        minimum_distinct_roots: policy.minimum_distinct_roots,
        actual_distinct_roots: root_counts.len(),
        required_distinct_roots: required_roots,
        root_density_within_limit: root_counts
            .values()
            .all(|count| *count <= policy.maximum_members_per_root),
    };
    if !assessment.is_valid() {
        return Err(FleetRegistryOpsError::FleetServicePlacementMismatch {
            service: service.service.clone(),
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct FleetServicePurposeCounts {
    authorities: u32,
    replicas: u32,
    pool_members: u32,
}

impl FleetServicePurposeCounts {
    fn record(&mut self, purpose: FleetServiceMemberPurpose) -> Option<()> {
        let count = match purpose {
            FleetServiceMemberPurpose::Authority => &mut self.authorities,
            FleetServiceMemberPurpose::Replica => &mut self.replicas,
            FleetServiceMemberPurpose::PoolMember => &mut self.pool_members,
        };
        *count = count.checked_add(1)?;
        Some(())
    }

    const fn matches(self, mode: FleetServiceMode) -> bool {
        match mode {
            FleetServiceMode::AuthorityReplica => self.authorities == 1 && self.pool_members == 0,
            FleetServiceMode::ActivePool => {
                self.authorities == 0 && self.replicas == 0 && self.pool_members > 0
            }
        }
    }
}

struct FleetServicePlacementAssessment {
    maximum_members_per_root: u32,
    minimum_distinct_roots: u32,
    actual_distinct_roots: usize,
    required_distinct_roots: usize,
    root_density_within_limit: bool,
}

impl FleetServicePlacementAssessment {
    const fn is_valid(&self) -> bool {
        self.maximum_members_per_root > 0
            && self.minimum_distinct_roots > 0
            && self.actual_distinct_roots >= self.required_distinct_roots
            && self.root_density_within_limit
    }
}

fn compare_service_members(
    left: &FleetServiceComponentBinding,
    right: &FleetServiceComponentBinding,
) -> std::cmp::Ordering {
    service_member_purpose_tag(left.member_purpose)
        .cmp(&service_member_purpose_tag(right.member_purpose))
        .then_with(|| left.group_placement.cmp(&right.group_placement))
        .then_with(|| left.member_path.cmp(&right.member_path))
        .then_with(|| left.component.cmp(&right.component))
}

const fn service_member_purpose_tag(purpose: FleetServiceMemberPurpose) -> u8 {
    match purpose {
        FleetServiceMemberPurpose::Authority => 0,
        FleetServiceMemberPurpose::Replica => 1,
        FleetServiceMemberPurpose::PoolMember => 2,
    }
}
