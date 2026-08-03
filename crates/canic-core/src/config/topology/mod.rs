//! Module: config::topology
//!
//! Responsibility: compile validated App configuration into canonical Component Topology.
//! Does not own: physical placement, root admission selection, Registry state, or lifecycle.
//! Boundary: emits bounded flat role catalogs, spawn grants, and stable protected identities.

#[cfg(test)]
mod tests;
mod validation;

use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, ComponentChildConfig, ComponentChildKind, ComponentSpecConfig,
        ConfigModel, CyclesFundingPolicyConfig, DiagnosticsCanisterConfig, IndexConfig,
        MAX_COMPONENT_PROVISIONING_GRANTS, MAX_COMPONENT_SPAWN_GRANTS, MetricsCanisterConfig,
        MetricsProfile, ScalePoolPolicy, ScalingConfig, ShardPoolPolicy, ShardingConfig,
        StandardsCanisterConfig, TopupPolicy,
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest,
        CyclesFundingBudget,
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error as ThisError;

const COMPONENT_SPEC_HASH_DOMAIN: &[u8] = b"canic/component-spec/v1";
const COMPONENT_TOPOLOGY_HASH_DOMAIN: &[u8] = b"canic/component-topology/v1";
const COMPONENT_TOPOLOGY_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes accepted for one Fleet-wide Component Topology.
pub const MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES: usize = 2_097_152;

impl ConfigModel {
    /// Compile this validated declaration model into its bounded canonical topology.
    pub fn compile_component_topology(&self) -> Result<ComponentTopology, ComponentTopologyError> {
        ComponentTopology::compile(self)
    }
}

///
/// ComponentTopology
///
/// Canonically ordered Fleet Component admissions and flat potential child-role catalogs.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentTopology {
    pub component_specs: Vec<ComponentSpec>,
    pub provisioning_grants: Vec<ComponentProvisioningGrant>,
}

fn validate_component_specs(specs: &[ComponentSpec]) -> Result<(), ComponentTopologyError> {
    let mut previous_component_spec: Option<&ComponentSpecId> = None;
    for spec in specs {
        if previous_component_spec.is_some_and(|previous| previous >= &spec.component_spec) {
            return Err(ComponentTopologyError::NonCanonicalComponentSpecOrder {
                component_spec: spec.component_spec.clone(),
            });
        }
        previous_component_spec = Some(&spec.component_spec);

        let mut previous_child: Option<&CanisterRole> = None;
        for child in &spec.children {
            if previous_child.is_some_and(|previous| previous >= &child.role) {
                return Err(ComponentTopologyError::NonCanonicalComponentChildOrder {
                    component_spec: spec.component_spec.clone(),
                    role: child.role.clone(),
                });
            }
            previous_child = Some(&child.role);
        }

        validate_spawn_grants(spec)?;
    }

    Ok(())
}

fn validate_spawn_grants(spec: &ComponentSpec) -> Result<(), ComponentTopologyError> {
    let mut previous: Option<(&CanisterRole, &CanisterRole)> = None;
    let mut incoming = BTreeSet::new();
    for grant in &spec.spawn_grants {
        let identity = (&grant.parent_role, &grant.child_role);
        if previous.is_some_and(|previous| previous >= identity) {
            return Err(ComponentTopologyError::NonCanonicalSpawnGrantOrder {
                component_spec: spec.component_spec.clone(),
                parent_role: grant.parent_role.clone(),
                child_role: grant.child_role.clone(),
            });
        }
        previous = Some(identity);

        validate_spawn_grant(spec, grant)?;
        incoming.insert(&grant.child_role);
    }
    if spec.spawn_grants.len() > MAX_COMPONENT_SPAWN_GRANTS {
        return Err(ComponentTopologyError::SpawnGrantBoundExceeded {
            component_spec: spec.component_spec.clone(),
            actual: spec.spawn_grants.len(),
            maximum: MAX_COMPONENT_SPAWN_GRANTS,
        });
    }
    for child in &spec.children {
        if !incoming.contains(&child.role) {
            return Err(ComponentTopologyError::ChildRoleWithoutSpawnGrant {
                component_spec: spec.component_spec.clone(),
                child_role: child.role.clone(),
            });
        }
    }

    Ok(())
}

fn validate_spawn_grant(
    spec: &ComponentSpec,
    grant: &ComponentSpawnGrant,
) -> Result<(), ComponentTopologyError> {
    if grant.maximum_instances_per_parent == 0 {
        return Err(ComponentTopologyError::ZeroSpawnGrantLimit {
            component_spec: spec.component_spec.clone(),
            parent_role: grant.parent_role.clone(),
            child_role: grant.child_role.clone(),
        });
    }
    if grant.parent_role != spec.component_role
        && spec
            .children
            .binary_search_by(|child| child.role.cmp(&grant.parent_role))
            .is_err()
    {
        return Err(ComponentTopologyError::UnknownSpawnGrantParent {
            component_spec: spec.component_spec.clone(),
            parent_role: grant.parent_role.clone(),
        });
    }
    let child = spec
        .children
        .binary_search_by(|child| child.role.cmp(&grant.child_role))
        .ok()
        .map(|index| &spec.children[index])
        .ok_or_else(|| ComponentTopologyError::UnknownSpawnGrantChild {
            component_spec: spec.component_spec.clone(),
            child_role: grant.child_role.clone(),
        })?;
    if child.kind == ComponentChildKind::Singleton && grant.maximum_instances_per_parent != 1 {
        return Err(ComponentTopologyError::InvalidSingletonSpawnGrantLimit {
            component_spec: spec.component_spec.clone(),
            parent_role: grant.parent_role.clone(),
            child_role: grant.child_role.clone(),
        });
    }

    Ok(())
}

impl ComponentTopology {
    /// Compile one validated config into canonical Component Spec order.
    pub fn compile(config: &ConfigModel) -> Result<Self, ComponentTopologyError> {
        let mut component_specs = Vec::with_capacity(config.component_specs.len());

        for (component_spec, source) in &config.component_specs {
            component_specs.push(compile_component_spec(config, component_spec, source)?);
        }

        let provisioning_grants = compile_provisioning_grants(config)?;
        let topology = Self {
            component_specs,
            provisioning_grants,
        };
        topology.canonical_bytes()?;
        Ok(topology)
    }

    /// Return the exact compiled Spec for one declaration ID.
    #[must_use]
    pub fn get(&self, component_spec: &ComponentSpecId) -> Option<&ComponentSpec> {
        self.component_specs
            .binary_search_by(|candidate| candidate.component_spec.cmp(component_spec))
            .ok()
            .map(|index| &self.component_specs[index])
    }

    /// Return one exact non-parent peer-Component authorization edge.
    #[must_use]
    pub fn provisioning_grant(
        &self,
        requester_component_spec: &ComponentSpecId,
        target_component_spec: &ComponentSpecId,
    ) -> Option<&ComponentProvisioningGrant> {
        self.provisioning_grants
            .binary_search_by(|grant| {
                (
                    &grant.requester_component_spec,
                    &grant.target_component_spec,
                )
                    .cmp(&(requester_component_spec, target_component_spec))
            })
            .ok()
            .map(|index| &self.provisioning_grants[index])
    }

    /// Encode the complete topology with its frozen canonical schema.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ComponentTopologyError> {
        self.validate_canonical_projection()?;

        let mut encoder = CanonicalEncoder::new(COMPONENT_TOPOLOGY_HASH_DOMAIN);
        encoder.u64(self.component_specs.len() as u64);

        for component_spec in &self.component_specs {
            encode_compiled_component_spec(&mut encoder, component_spec);
        }
        encoder.u64(self.provisioning_grants.len() as u64);

        for grant in &self.provisioning_grants {
            encode_provisioning_grant(&mut encoder, grant);
        }

        encoder.finish("Component Topology")
    }

    fn validate_canonical_projection(&self) -> Result<(), ComponentTopologyError> {
        validate_component_specs(&self.component_specs)?;

        let mut previous_grant: Option<(&ComponentSpecId, &ComponentSpecId)> = None;
        let mut grant_counts = BTreeMap::<ComponentSpecId, usize>::new();
        for grant in &self.provisioning_grants {
            let identity = (
                &grant.requester_component_spec,
                &grant.target_component_spec,
            );
            if previous_grant.is_some_and(|previous| previous >= identity) {
                return Err(ComponentTopologyError::NonCanonicalProvisioningGrantOrder {
                    requester_component_spec: grant.requester_component_spec.clone(),
                    target_component_spec: grant.target_component_spec.clone(),
                });
            }
            previous_grant = Some(identity);

            if grant.requester_component_spec == grant.target_component_spec {
                return Err(ComponentTopologyError::SelfProvisioningGrant {
                    requester_component_spec: grant.requester_component_spec.clone(),
                    target_component_spec: grant.target_component_spec.clone(),
                });
            }
            if grant.maximum_instances_per_requester_per_root == 0 {
                return Err(ComponentTopologyError::ZeroProvisioningGrantLimit {
                    requester_component_spec: grant.requester_component_spec.clone(),
                    target_component_spec: grant.target_component_spec.clone(),
                });
            }
            if self.get(&grant.target_component_spec).is_none() {
                return Err(ComponentTopologyError::UnknownProvisioningGrantTarget {
                    requester_component_spec: grant.requester_component_spec.clone(),
                    target_component_spec: grant.target_component_spec.clone(),
                });
            }

            let count = grant_counts
                .entry(grant.requester_component_spec.clone())
                .or_default();
            *count += 1;
            if *count > MAX_COMPONENT_PROVISIONING_GRANTS {
                return Err(ComponentTopologyError::ProvisioningGrantBoundExceeded {
                    requester_component_spec: grant.requester_component_spec.clone(),
                    actual: *count,
                    maximum: MAX_COMPONENT_PROVISIONING_GRANTS,
                });
            }
        }

        validate_projected_grant_cycles(self)
    }

    /// Compute the digest of this exact canonical topology projection.
    pub fn digest(&self) -> Result<ComponentTopologyDigest, ComponentTopologyError> {
        let bytes = self.canonical_bytes()?;
        Ok(ComponentTopologyDigest::from_bytes(
            Sha256::digest(bytes).into(),
        ))
    }

    /// Validate canonical root admissions and project their exact Spec closure.
    pub fn project_for_admissions(
        &self,
        admissions: &[ComponentSpecAdmission],
    ) -> Result<Self, ComponentTopologyError> {
        if admissions.is_empty() {
            return Err(ComponentTopologyError::EmptyRootAdmissions);
        }

        let mut previous: Option<&ComponentSpecId> = None;
        let mut component_specs = Vec::with_capacity(admissions.len());

        for admission in admissions {
            if let Some(previous) = previous
                && previous >= &admission.component_spec
            {
                return Err(ComponentTopologyError::NonCanonicalAdmissionOrder {
                    previous: previous.clone(),
                    current: admission.component_spec.clone(),
                });
            }
            previous = Some(&admission.component_spec);

            if admission.maximum_root_instances == 0 {
                return Err(ComponentTopologyError::ZeroRootAdmission {
                    component_spec: admission.component_spec.clone(),
                });
            }

            let component_spec = self.get(&admission.component_spec).ok_or_else(|| {
                ComponentTopologyError::UnknownAdmissionSpec {
                    component_spec: admission.component_spec.clone(),
                }
            })?;
            if admission.spec_hash != component_spec.spec_hash {
                return Err(ComponentTopologyError::AdmissionSpecHashMismatch {
                    component_spec: admission.component_spec.clone(),
                    expected: component_spec.spec_hash,
                    received: admission.spec_hash,
                });
            }
            if admission.maximum_root_instances > component_spec.maximum_fleet_instances {
                return Err(ComponentTopologyError::RootAdmissionExceedsFleetMaximum {
                    component_spec: admission.component_spec.clone(),
                    maximum_root_instances: admission.maximum_root_instances,
                    maximum_fleet_instances: component_spec.maximum_fleet_instances,
                });
            }

            component_specs.push(component_spec.clone());
        }

        let admitted_component_specs = component_specs
            .iter()
            .map(|spec| spec.component_spec.clone())
            .collect::<BTreeSet<_>>();
        let provisioning_grants = self
            .provisioning_grants
            .iter()
            .filter(|grant| admitted_component_specs.contains(&grant.target_component_spec))
            .cloned()
            .collect();
        let projected = Self {
            component_specs,
            provisioning_grants,
        };
        projected.canonical_bytes()?;
        Ok(projected)
    }

    /// Validate complete Fleet admission coverage and per-Spec sums.
    pub fn validate_fleet_admissions(
        &self,
        root_admissions: &[&[ComponentSpecAdmission]],
    ) -> Result<(), ComponentTopologyError> {
        let mut totals = self
            .component_specs
            .iter()
            .map(|spec| (spec.component_spec.clone(), 0_u32))
            .collect::<BTreeMap<_, _>>();

        for admissions in root_admissions {
            self.project_for_admissions(admissions)?;
            for admission in *admissions {
                let total = totals.get_mut(&admission.component_spec).ok_or_else(|| {
                    ComponentTopologyError::UnknownAdmissionSpec {
                        component_spec: admission.component_spec.clone(),
                    }
                })?;
                *total = total
                    .checked_add(admission.maximum_root_instances)
                    .ok_or_else(|| ComponentTopologyError::FleetAdmissionOverflow {
                        component_spec: admission.component_spec.clone(),
                    })?;
            }
        }

        for component_spec in &self.component_specs {
            let admitted = totals[&component_spec.component_spec];
            if admitted == 0 {
                return Err(ComponentTopologyError::MissingFleetAdmission {
                    component_spec: component_spec.component_spec.clone(),
                });
            }
            if admitted > component_spec.maximum_fleet_instances {
                return Err(ComponentTopologyError::FleetAdmissionsExceedMaximum {
                    component_spec: component_spec.component_spec.clone(),
                    admitted,
                    maximum_fleet_instances: component_spec.maximum_fleet_instances,
                });
            }
        }

        Ok(())
    }
}

///
/// ComponentSpec
///
/// One compiled Component role, immutable Spec hash, limits, and child-role catalog.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpec {
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub component_role: CanisterRole,
    pub maximum_fleet_instances: u32,
    pub limits: ComponentLimits,
    pub children: Vec<ComponentChildSpec>,
    pub spawn_grants: Vec<ComponentSpawnGrant>,
}

impl ComponentSpec {
    /// Return one admitted potential descendant role.
    #[must_use]
    pub fn child(&self, role: &CanisterRole) -> Option<&ComponentChildSpec> {
        self.children
            .binary_search_by(|child| child.role.cmp(role))
            .ok()
            .map(|index| &self.children[index])
    }

    /// Return the exact capability for one registered parent role to create one child role.
    #[must_use]
    pub fn spawn_grant(
        &self,
        parent_role: &CanisterRole,
        child_role: &CanisterRole,
    ) -> Option<&ComponentSpawnGrant> {
        self.spawn_grants
            .binary_search_by(|grant| {
                (&grant.parent_role, &grant.child_role).cmp(&(parent_role, child_role))
            })
            .ok()
            .map(|index| &self.spawn_grants[index])
    }
}

///
/// ComponentLimits
///
/// Explicit aggregate quotas compiled for every concrete instance of one Spec.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentLimits {
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub cycles_funding: CyclesFundingBudget,
}

///
/// ComponentChildSpec
///
/// One canonically ordered role permitted anywhere below a Component.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildSpec {
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub cycles_funding: ComponentChildFundingPolicy,
}

///
/// ComponentSpawnGrant
///
/// One explicit parent-role capability to create one direct child role.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpawnGrant {
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub maximum_instances_per_parent: u32,
}

///
/// ComponentProvisioningGrant
///
/// Non-parent authorization for one requester Spec to create one exact peer Spec.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentProvisioningGrant {
    pub requester_component_spec: ComponentSpecId,
    pub target_component_spec: ComponentSpecId,
    pub maximum_instances_per_requester_per_root: u32,
}

///
/// ComponentChildFundingPolicy
///
/// Compiled child-level funding limits without configuration-only deserializers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildFundingPolicy {
    pub max_per_request: Cycles,
    pub max_per_child: Cycles,
    pub cooldown_secs: u64,
}

///
/// ComponentTopologyError
///
/// Typed failure while compiling, encoding, or projecting Component Topology.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentTopologyError {
    #[error("Component Spec '{component_spec}' admission hash does not match compiled topology")]
    AdmissionSpecHashMismatch {
        component_spec: ComponentSpecId,
        expected: [u8; 32],
        received: [u8; 32],
    },

    #[error("{subject} canonical encoding is {actual_bytes} bytes, exceeding {maximum_bytes}")]
    CanonicalBytesExceeded {
        subject: &'static str,
        actual_bytes: usize,
        maximum_bytes: usize,
    },

    #[error("protected binding field '{field}' must not use the anonymous principal")]
    AnonymousBindingPrincipal { field: &'static str },

    #[error("Component binding authority does not match its Fleet Subnet Root")]
    BindingAuthorityMismatch,

    #[error(
        "Component binding role '{received}' does not match Spec '{component_spec}' role '{expected}'"
    )]
    BindingComponentRoleMismatch {
        component_spec: ComponentSpecId,
        expected: CanisterRole,
        received: CanisterRole,
    },

    #[error("Component binding placement Subnet does not match its Fleet Subnet Root")]
    BindingPlacementSubnetMismatch,

    #[error("Component binding root principal does not match its Fleet Subnet Root")]
    BindingRootMismatch,

    #[error("Component binding hash does not match Spec '{component_spec}'")]
    BindingSpecHashMismatch { component_spec: ComponentSpecId },

    #[error("Component Child role '{role}' is not admitted by Spec '{component_spec}'")]
    ChildRoleNotAdmitted {
        component_spec: ComponentSpecId,
        role: CanisterRole,
    },

    #[error(
        "protected Component Child principal must differ from its Component, parent, root and Coordinator"
    )]
    ChildPrincipalConflictsWithOwner,

    #[error("protected Component Child parent must differ from its Coordinator and root")]
    ChildParentConflictsWithAuthority,

    #[error("protected Component principal must differ from its Coordinator and root")]
    ComponentPrincipalConflictsWithAuthority,

    #[error("Fleet Subnet Root set contains duplicate root principal {fleet_subnet_root}")]
    DuplicateFleetSubnetRootPrincipal {
        fleet_subnet_root: candid::Principal,
    },

    #[error("Fleet Subnet Root set contains duplicate placement Subnet {placement_subnet}")]
    DuplicateFleetSubnetRootSubnet {
        placement_subnet: crate::ids::SubnetId,
    },

    #[error("Component Spec '{component_spec}' Fleet admission sum overflowed")]
    FleetAdmissionOverflow { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{component_spec}' Fleet admission sum {admitted} exceeds maximum {maximum_fleet_instances}"
    )]
    FleetAdmissionsExceedMaximum {
        component_spec: ComponentSpecId,
        admitted: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Component Spec '{component_spec}' has no positive Fleet Subnet Root admission")]
    MissingFleetAdmission { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{requester_component_spec}' declares {actual} provisioning grants, exceeding bound {maximum}"
    )]
    ProvisioningGrantBoundExceeded {
        requester_component_spec: ComponentSpecId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Spec '{requester_component_spec}' cannot provision itself as peer Spec '{target_component_spec}'"
    )]
    SelfProvisioningGrant {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },

    #[error("Component provisioning grants contain a cycle involving Spec '{component_spec}'")]
    CyclicProvisioningGrant { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{requester_component_spec}' provisioning grant references unknown target '{target_component_spec}'"
    )]
    UnknownProvisioningGrantTarget {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },

    #[error(
        "Component Spec '{requester_component_spec}' provisioning grant to '{target_component_spec}' must have a positive per-requester/root ceiling"
    )]
    ZeroProvisioningGrantLimit {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },

    #[error("role '{role}' has no package declaration while compiling Component Topology")]
    MissingRoleDeclaration { role: CanisterRole },

    #[error("Fleet Subnet Root limit '{field}' must be positive")]
    NonPositiveRootLimit { field: &'static str },

    #[error(
        "Fleet Subnet Root Canister pool maximum_size {maximum_size} is smaller than minimum_size {minimum_size}"
    )]
    InvalidRootCanisterPoolRange {
        minimum_size: u32,
        maximum_size: u32,
    },

    #[error(
        "Fleet Subnet Root Canister pool maximum_size {maximum_size} exceeds maximum_managed_canisters {maximum_managed_canisters}"
    )]
    RootCanisterPoolExceedsManagedLimit {
        maximum_size: u32,
        maximum_managed_canisters: u32,
    },

    #[error(
        "root Component admissions are not in strict Component Spec order: '{previous}' before '{current}'"
    )]
    NonCanonicalAdmissionOrder {
        previous: ComponentSpecId,
        current: ComponentSpecId,
    },

    #[error("Component Specs are not in strict canonical order at '{component_spec}'")]
    NonCanonicalComponentSpecOrder { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{component_spec}' children are not in strict canonical order at role '{role}'"
    )]
    NonCanonicalComponentChildOrder {
        component_spec: ComponentSpecId,
        role: CanisterRole,
    },

    #[error(
        "Component provisioning grants are not in strict canonical order at '{requester_component_spec}' -> '{target_component_spec}'"
    )]
    NonCanonicalProvisioningGrantOrder {
        requester_component_spec: ComponentSpecId,
        target_component_spec: ComponentSpecId,
    },

    #[error(
        "Component Spec '{component_spec}' root admission {maximum_root_instances} exceeds Fleet maximum {maximum_fleet_instances}"
    )]
    RootAdmissionExceedsFleetMaximum {
        component_spec: ComponentSpecId,
        maximum_root_instances: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Fleet Subnet Root authority does not match the other roots in this Fleet plan")]
    RootAuthorityMismatch,

    #[error("Fleet Subnet Root principal must differ from its Coordinator principal")]
    RootPrincipalConflictsWithCoordinator,

    #[error("Fleet Subnet Root topology digest does not match its exact admitted projection")]
    RootTopologyDigestMismatch {
        expected: ComponentTopologyDigest,
        received: ComponentTopologyDigest,
    },

    #[error("Fleet Subnet Root must contain at least one Component Spec admission")]
    EmptyRootAdmissions,

    #[error("root admission references unknown Component Spec '{component_spec}'")]
    UnknownAdmissionSpec { component_spec: ComponentSpecId },

    #[error("Component Spec '{component_spec}' root admission must be positive")]
    ZeroRootAdmission { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{component_spec}' spawn grant '{parent_role}' -> '{child_role}' must have a positive per-parent ceiling"
    )]
    ZeroSpawnGrantLimit {
        component_spec: ComponentSpecId,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Spec '{component_spec}' singleton spawn grant '{parent_role}' -> '{child_role}' must have a per-parent ceiling of one"
    )]
    InvalidSingletonSpawnGrantLimit {
        component_spec: ComponentSpecId,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Spec '{component_spec}' spawn grant references undeclared parent role '{parent_role}'"
    )]
    UnknownSpawnGrantParent {
        component_spec: ComponentSpecId,
        parent_role: CanisterRole,
    },

    #[error(
        "Component Spec '{component_spec}' spawn grant references undeclared child role '{child_role}'"
    )]
    UnknownSpawnGrantChild {
        component_spec: ComponentSpecId,
        child_role: CanisterRole,
    },

    #[error(
        "Component Spec '{component_spec}' child role '{child_role}' has no incoming spawn grant"
    )]
    ChildRoleWithoutSpawnGrant {
        component_spec: ComponentSpecId,
        child_role: CanisterRole,
    },

    #[error(
        "Component Spec '{component_spec}' declares {actual} spawn grants, exceeding bound {maximum}"
    )]
    SpawnGrantBoundExceeded {
        component_spec: ComponentSpecId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Spec '{component_spec}' spawn grants are not in strict canonical order at '{parent_role}' -> '{child_role}'"
    )]
    NonCanonicalSpawnGrantOrder {
        component_spec: ComponentSpecId,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },
}

fn compile_provisioning_grants(
    config: &ConfigModel,
) -> Result<Vec<ComponentProvisioningGrant>, ComponentTopologyError> {
    let mut outgoing = config
        .component_specs
        .keys()
        .cloned()
        .map(|component_spec| (component_spec, Vec::new()))
        .collect::<BTreeMap<ComponentSpecId, Vec<ComponentSpecId>>>();
    let mut incoming = config
        .component_specs
        .keys()
        .cloned()
        .map(|component_spec| (component_spec, 0_u32))
        .collect::<BTreeMap<_, _>>();
    let mut grants = Vec::new();

    for (requester_component_spec, source) in &config.component_specs {
        if source.provisions.len() > MAX_COMPONENT_PROVISIONING_GRANTS {
            return Err(ComponentTopologyError::ProvisioningGrantBoundExceeded {
                requester_component_spec: requester_component_spec.clone(),
                actual: source.provisions.len(),
                maximum: MAX_COMPONENT_PROVISIONING_GRANTS,
            });
        }

        for (target_component_spec, grant) in &source.provisions {
            if requester_component_spec == target_component_spec {
                return Err(ComponentTopologyError::SelfProvisioningGrant {
                    requester_component_spec: requester_component_spec.clone(),
                    target_component_spec: target_component_spec.clone(),
                });
            }
            if grant.maximum_instances_per_requester_per_root == 0 {
                return Err(ComponentTopologyError::ZeroProvisioningGrantLimit {
                    requester_component_spec: requester_component_spec.clone(),
                    target_component_spec: target_component_spec.clone(),
                });
            }
            let Some(target_incoming) = incoming.get_mut(target_component_spec) else {
                return Err(ComponentTopologyError::UnknownProvisioningGrantTarget {
                    requester_component_spec: requester_component_spec.clone(),
                    target_component_spec: target_component_spec.clone(),
                });
            };
            *target_incoming = target_incoming.checked_add(1).ok_or_else(|| {
                ComponentTopologyError::ProvisioningGrantBoundExceeded {
                    requester_component_spec: requester_component_spec.clone(),
                    actual: source.provisions.len(),
                    maximum: MAX_COMPONENT_PROVISIONING_GRANTS,
                }
            })?;
            outgoing
                .get_mut(requester_component_spec)
                .expect("requester was initialized from the same Component Spec map")
                .push(target_component_spec.clone());
            grants.push(ComponentProvisioningGrant {
                requester_component_spec: requester_component_spec.clone(),
                target_component_spec: target_component_spec.clone(),
                maximum_instances_per_requester_per_root: grant
                    .maximum_instances_per_requester_per_root,
            });
        }
    }

    let mut ready = incoming
        .iter()
        .filter(|(_component_spec, count)| **count == 0)
        .map(|(component_spec, _count)| component_spec.clone())
        .collect::<VecDeque<_>>();
    let mut visited = 0_usize;

    while let Some(requester_component_spec) = ready.pop_front() {
        visited += 1;
        for target_component_spec in &outgoing[&requester_component_spec] {
            let target_incoming = incoming
                .get_mut(target_component_spec)
                .expect("grant targets were validated above");
            *target_incoming -= 1;
            if *target_incoming == 0 {
                ready.push_back(target_component_spec.clone());
            }
        }
    }

    if visited != config.component_specs.len() {
        let component_spec = incoming
            .into_iter()
            .find(|(_component_spec, count)| *count > 0)
            .map(|(component_spec, _count)| component_spec)
            .expect("an unvisited graph node must retain an incoming edge");
        return Err(ComponentTopologyError::CyclicProvisioningGrant { component_spec });
    }

    Ok(grants)
}

fn validate_projected_grant_cycles(
    topology: &ComponentTopology,
) -> Result<(), ComponentTopologyError> {
    let mut outgoing = topology
        .component_specs
        .iter()
        .map(|spec| (spec.component_spec.clone(), Vec::new()))
        .collect::<BTreeMap<ComponentSpecId, Vec<ComponentSpecId>>>();
    let mut incoming = topology
        .component_specs
        .iter()
        .map(|spec| (spec.component_spec.clone(), 0_u32))
        .collect::<BTreeMap<_, _>>();

    for grant in &topology.provisioning_grants {
        let Some(requester_outgoing) = outgoing.get_mut(&grant.requester_component_spec) else {
            continue;
        };
        requester_outgoing.push(grant.target_component_spec.clone());
        *incoming
            .get_mut(&grant.target_component_spec)
            .expect("grant targets were validated before cycle detection") += 1;
    }

    let mut ready = incoming
        .iter()
        .filter(|(_component_spec, count)| **count == 0)
        .map(|(component_spec, _count)| component_spec.clone())
        .collect::<VecDeque<_>>();
    let mut visited = 0_usize;
    while let Some(requester_component_spec) = ready.pop_front() {
        visited += 1;
        for target_component_spec in &outgoing[&requester_component_spec] {
            let target_incoming = incoming
                .get_mut(target_component_spec)
                .expect("grant targets were validated before cycle detection");
            *target_incoming -= 1;
            if *target_incoming == 0 {
                ready.push_back(target_component_spec.clone());
            }
        }
    }

    if visited != topology.component_specs.len() {
        let component_spec = incoming
            .into_iter()
            .find(|(_component_spec, count)| *count > 0)
            .map(|(component_spec, _count)| component_spec)
            .expect("an unvisited graph node must retain an incoming edge");
        return Err(ComponentTopologyError::CyclicProvisioningGrant { component_spec });
    }

    Ok(())
}

fn compile_component_spec(
    config: &ConfigModel,
    component_spec: &ComponentSpecId,
    source: &ComponentSpecConfig,
) -> Result<ComponentSpec, ComponentTopologyError> {
    let component_package = role_package(config, &source.component_role)?;
    let spec_hash = component_spec_hash(config, source, component_package)?;
    let children = source
        .children
        .iter()
        .map(|(role, child)| ComponentChildSpec {
            role: role.clone(),
            kind: child.kind,
            cycles_funding: ComponentChildFundingPolicy {
                max_per_request: child.cycles_funding.max_per_request.clone(),
                max_per_child: child.cycles_funding.max_per_child.clone(),
                cooldown_secs: child.cycles_funding.cooldown_secs,
            },
        })
        .collect();
    let spawn_grants = source
        .spawn_grants
        .iter()
        .flat_map(|(parent_role, grants)| {
            grants
                .iter()
                .map(move |(child_role, grant)| ComponentSpawnGrant {
                    parent_role: parent_role.clone(),
                    child_role: child_role.clone(),
                    maximum_instances_per_parent: grant.maximum_instances_per_parent,
                })
        })
        .collect();

    Ok(ComponentSpec {
        component_spec: component_spec.clone(),
        spec_hash,
        component_role: source.component_role.clone(),
        maximum_fleet_instances: source.maximum_instances,
        limits: ComponentLimits {
            maximum_descendants: source.limits.maximum_descendants,
            maximum_registry_bytes: source.limits.maximum_registry_bytes,
            cycles_funding: CyclesFundingBudget {
                window_secs: source.limits.cycles_funding.window_secs,
                maximum_cycles: source.limits.cycles_funding.maximum_cycles.clone(),
            },
        },
        children,
        spawn_grants,
    })
}

fn component_spec_hash(
    config: &ConfigModel,
    source: &ComponentSpecConfig,
    component_package: &str,
) -> Result<[u8; 32], ComponentTopologyError> {
    let mut encoder = CanonicalEncoder::new(COMPONENT_SPEC_HASH_DOMAIN);

    encoder.string(source.component_role.as_str());
    encoder.string(component_package);
    encoder.u32(source.maximum_instances);
    encode_component_limits(&mut encoder, source);
    encode_component_runtime_policy(&mut encoder, source);
    encode_scaling(&mut encoder, source.scaling.as_ref());
    encode_sharding(&mut encoder, source.sharding.as_ref());
    encode_index(&mut encoder, source.index.as_ref());
    encoder.u64(source.children.len() as u64);

    for (role, child) in &source.children {
        encoder.string(role.as_str());
        encoder.string(role_package(config, role)?);
        encode_component_child(&mut encoder, child);
    }
    encode_source_spawn_grants(&mut encoder, source);

    let bytes = encoder.finish("Component Spec")?;
    Ok(Sha256::digest(bytes).into())
}

fn role_package<'a>(
    config: &'a ConfigModel,
    role: &CanisterRole,
) -> Result<&'a str, ComponentTopologyError> {
    config
        .roles
        .get(role)
        .map(|declaration| declaration.package.as_str())
        .ok_or_else(|| ComponentTopologyError::MissingRoleDeclaration { role: role.clone() })
}

fn encode_compiled_component_spec(encoder: &mut CanonicalEncoder, spec: &ComponentSpec) {
    encoder.string(spec.component_spec.as_str());
    encoder.bytes(&spec.spec_hash);
    encoder.string(spec.component_role.as_str());
    encoder.u32(spec.maximum_fleet_instances);
    encoder.u32(spec.limits.maximum_descendants);
    encoder.u64(spec.limits.maximum_registry_bytes);
    encode_cycles_budget(encoder, &spec.limits.cycles_funding);
    encoder.u64(spec.children.len() as u64);

    for child in &spec.children {
        encoder.string(child.role.as_str());
        encoder.u8(component_child_kind_tag(child.kind));
        encode_compiled_child_funding_policy(encoder, &child.cycles_funding);
    }
    encoder.u64(spec.spawn_grants.len() as u64);
    for grant in &spec.spawn_grants {
        encoder.string(grant.parent_role.as_str());
        encoder.string(grant.child_role.as_str());
        encoder.u32(grant.maximum_instances_per_parent);
    }
}

fn encode_provisioning_grant(encoder: &mut CanonicalEncoder, grant: &ComponentProvisioningGrant) {
    encoder.string(grant.requester_component_spec.as_str());
    encoder.string(grant.target_component_spec.as_str());
    encoder.u32(grant.maximum_instances_per_requester_per_root);
}

fn encode_component_limits(encoder: &mut CanonicalEncoder, source: &ComponentSpecConfig) {
    encoder.u32(source.limits.maximum_descendants);
    encoder.u64(source.limits.maximum_registry_bytes);
    encoder.u64(source.limits.cycles_funding.window_secs);
    encoder.u128(source.limits.cycles_funding.maximum_cycles.to_u128());
}

fn encode_component_runtime_policy(encoder: &mut CanonicalEncoder, source: &ComponentSpecConfig) {
    encoder.u128(source.initial_cycles.to_u128());
    encode_topup(encoder, source.topup.as_ref());
    encode_cycles_funding_policy(encoder, &source.cycles_funding);
    encode_auth(encoder, &source.auth);
    encode_standards(encoder, &source.standards);
    encode_diagnostics(encoder, source.diagnostics);
    encode_metrics(encoder, source.metrics);
}

fn encode_component_child(encoder: &mut CanonicalEncoder, child: &ComponentChildConfig) {
    encoder.u8(component_child_kind_tag(child.kind));
    encoder.u128(child.initial_cycles.to_u128());
    encode_topup(encoder, child.topup.as_ref());
    encode_cycles_funding_policy(encoder, &child.cycles_funding);
    encode_scaling(encoder, child.scaling.as_ref());
    encode_sharding(encoder, child.sharding.as_ref());
    encode_index(encoder, child.index.as_ref());
    encode_auth(encoder, &child.auth);
    encode_standards(encoder, &child.standards);
    encode_diagnostics(encoder, child.diagnostics);
    encode_metrics(encoder, child.metrics);
}

fn encode_source_spawn_grants(encoder: &mut CanonicalEncoder, source: &ComponentSpecConfig) {
    let grant_count = source
        .spawn_grants
        .values()
        .map(BTreeMap::len)
        .sum::<usize>();
    encoder.u64(grant_count as u64);
    for (parent_role, grants) in &source.spawn_grants {
        for (child_role, grant) in grants {
            encoder.string(parent_role.as_str());
            encoder.string(child_role.as_str());
            encoder.u32(grant.maximum_instances_per_parent);
        }
    }
}

fn encode_cycles_budget(encoder: &mut CanonicalEncoder, budget: &CyclesFundingBudget) {
    encoder.u64(budget.window_secs);
    encoder.u128(budget.maximum_cycles.to_u128());
}

fn encode_cycles_funding_policy(
    encoder: &mut CanonicalEncoder,
    policy: &CyclesFundingPolicyConfig,
) {
    encoder.u128(policy.max_per_request.to_u128());
    encoder.u128(policy.max_per_child.to_u128());
    encoder.u64(policy.cooldown_secs);
}

fn encode_compiled_child_funding_policy(
    encoder: &mut CanonicalEncoder,
    policy: &ComponentChildFundingPolicy,
) {
    encoder.u128(policy.max_per_request.to_u128());
    encoder.u128(policy.max_per_child.to_u128());
    encoder.u64(policy.cooldown_secs);
}

fn encode_topup(encoder: &mut CanonicalEncoder, topup: Option<&TopupPolicy>) {
    let Some(topup) = topup else {
        encoder.u8(0);
        return;
    };

    encoder.u8(1);
    encoder.u128(topup.threshold.to_u128());
    encoder.u128(topup.amount.to_u128());
}

fn encode_scaling(encoder: &mut CanonicalEncoder, scaling: Option<&ScalingConfig>) {
    let pools = scaling.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(scaling) = scaling {
        for (name, pool) in &scaling.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encode_scale_pool_policy(encoder, &pool.policy);
        }
    }
}

fn encode_scale_pool_policy(encoder: &mut CanonicalEncoder, policy: &ScalePoolPolicy) {
    encoder.u32(policy.initial_workers);
    encoder.u32(policy.min_workers);
    encoder.u32(policy.max_workers);
}

fn encode_sharding(encoder: &mut CanonicalEncoder, sharding: Option<&ShardingConfig>) {
    let pools = sharding.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(sharding) = sharding {
        for (name, pool) in &sharding.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encode_shard_pool_policy(encoder, &pool.policy);
        }
    }
}

fn encode_shard_pool_policy(encoder: &mut CanonicalEncoder, policy: &ShardPoolPolicy) {
    encoder.u32(policy.capacity);
    encoder.u32(policy.initial_shards);
    encoder.u32(policy.max_shards);
}

fn encode_index(encoder: &mut CanonicalEncoder, index: Option<&IndexConfig>) {
    let pools = index.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(index) = index {
        for (name, pool) in &index.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encoder.string(&pool.key_name);
        }
    }
}

fn encode_auth(encoder: &mut CanonicalEncoder, auth: &CanisterAuthConfig) {
    encoder.boolean(auth.delegated_token_issuer);
    encoder.boolean(auth.delegated_token_verifier);
    encoder.boolean(auth.role_attestation_cache);
}

fn encode_standards(encoder: &mut CanonicalEncoder, standards: &StandardsCanisterConfig) {
    encoder.boolean(standards.icrc21);
}

fn encode_diagnostics(encoder: &mut CanonicalEncoder, diagnostics: DiagnosticsCanisterConfig) {
    encoder.boolean(diagnostics.memory_ledger);
}

fn encode_metrics(encoder: &mut CanonicalEncoder, metrics: MetricsCanisterConfig) {
    match metrics.profile {
        None => encoder.u8(0),
        Some(profile) => {
            encoder.u8(1);
            encoder.u8(metrics_profile_tag(profile));
        }
    }
}

const fn component_child_kind_tag(kind: ComponentChildKind) -> u8 {
    match kind {
        ComponentChildKind::Singleton => 0,
        ComponentChildKind::Replica => 1,
        ComponentChildKind::Shard => 2,
        ComponentChildKind::Instance => 3,
    }
}

const fn metrics_profile_tag(profile: MetricsProfile) -> u8 {
    match profile {
        MetricsProfile::Leaf => 0,
        MetricsProfile::Hub => 1,
        MetricsProfile::Storage => 2,
        MetricsProfile::Root => 3,
        MetricsProfile::Full => 4,
    }
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(COMPONENT_TOPOLOGY_SCHEMA_VERSION);
        encoder
    }

    fn boolean(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self, subject: &'static str) -> Result<Vec<u8>, ComponentTopologyError> {
        if self.bytes.len() > MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES {
            return Err(ComponentTopologyError::CanonicalBytesExceeded {
                subject,
                actual_bytes: self.bytes.len(),
                maximum_bytes: MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES,
            });
        }
        Ok(self.bytes)
    }
}
