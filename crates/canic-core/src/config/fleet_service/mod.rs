//! Module: config::fleet_service
//!
//! Responsibility: compile Fleet-service targets against exact flattened deployment occurrences.
//! Does not own: concrete root assignment, Component identity, publication, or runtime state.
//! Boundary: checked-in targets become canonical mode-compatible logical service topology.

mod canonical;
#[cfg(test)]
mod tests;

use crate::{
    config::{
        ComponentDeploymentPurpose, ComponentGroupDeploymentTopology,
        ComponentGroupDeploymentTopologyError, ComponentTopology, ComponentTopologyError,
        FleetServiceMemberPurpose,
        schema::{ConfigModel, FleetServicePlacementPolicyConfig, FleetServiceTargetConfig},
    },
    ids::{
        CanisterRole, ComponentGroupDeploymentId, ComponentGroupMemberPath, ComponentSpecId,
        FleetServiceId,
    },
};
use std::collections::BTreeMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use canonical::MAX_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES;

/// Maximum logical Fleet-service targets in one App configuration.
pub const MAX_FLEET_SERVICE_TARGETS: usize = 4_096;

impl ConfigModel {
    /// Compile every Fleet service against the exact flattened deployment topology.
    pub fn compile_fleet_service_topology(
        &self,
    ) -> Result<FleetServiceTopology, FleetServiceTopologyError> {
        FleetServiceTopology::compile(self)
    }
}

/// Canonical Fleet-service targets in raw service-ID order.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServiceTopology {
    pub targets: Vec<FleetServiceTarget>,
}

impl FleetServiceTopology {
    /// Compile strict source targets and validate their complete occurrence relationship.
    pub fn compile(config: &ConfigModel) -> Result<Self, FleetServiceTopologyError> {
        let component_topology = config.compile_component_topology()?;
        let deployment_topology = config.compile_component_group_deployment_topology()?;
        Self::compile_from_topologies(config, &deployment_topology, &component_topology)
    }

    /// Compile from the exact Component and deployment projections already validated by config.
    pub(super) fn compile_from_topologies(
        config: &ConfigModel,
        deployment_topology: &ComponentGroupDeploymentTopology,
        component_topology: &ComponentTopology,
    ) -> Result<Self, FleetServiceTopologyError> {
        validate_target_count(config.services.fleet.targets.len())?;
        let targets = config
            .services
            .fleet
            .targets
            .iter()
            .map(|(service, source)| compile_target(service, source))
            .collect();
        let topology = Self { targets };
        topology.validate_relationships(deployment_topology, component_topology)?;
        Ok(topology)
    }

    /// Return one exact canonical service target.
    #[must_use]
    pub fn get(&self, service: &FleetServiceId) -> Option<&FleetServiceTarget> {
        self.targets
            .binary_search_by(|candidate| candidate.service.cmp(service))
            .ok()
            .map(|index| &self.targets[index])
    }

    /// Revalidate decoded service topology against current compiled declarations.
    pub fn validate(
        &self,
        config: &ConfigModel,
        deployment_topology: &ComponentGroupDeploymentTopology,
        component_topology: &ComponentTopology,
    ) -> Result<(), FleetServiceTopologyError> {
        validate_target_count(config.services.fleet.targets.len())?;
        let component_group_topology = config
            .compile_component_group_topology()
            .map_err(ComponentGroupDeploymentTopologyError::ComponentGroupTopology)?;
        deployment_topology.validate(&component_group_topology, component_topology)?;
        self.validate_relationships(deployment_topology, component_topology)?;
        let expected_targets = config
            .services
            .fleet
            .targets
            .iter()
            .map(|(service, source)| compile_target(service, source))
            .collect::<Vec<_>>();
        if self.targets != expected_targets {
            return Err(FleetServiceTopologyError::TargetProjectionMismatch);
        }
        Ok(())
    }

    /// Return the exact canonical Fleet-service target section for semantic hashing.
    pub fn canonical_bytes(
        &self,
        deployment_topology: &ComponentGroupDeploymentTopology,
        component_topology: &ComponentTopology,
    ) -> Result<Vec<u8>, FleetServiceTopologyError> {
        self.validate_relationships(deployment_topology, component_topology)?;
        let bytes = canonical::encode(self);
        if bytes.len() > MAX_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES {
            return Err(FleetServiceTopologyError::CanonicalBytesBoundExceeded {
                actual: bytes.len(),
                maximum: MAX_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES,
            });
        }
        Ok(bytes)
    }

    fn validate_relationships(
        &self,
        deployment_topology: &ComponentGroupDeploymentTopology,
        component_topology: &ComponentTopology,
    ) -> Result<(), FleetServiceTopologyError> {
        component_topology.canonical_bytes()?;
        validate_target_count(self.targets.len())?;
        let mut occurrences = service_occurrences(deployment_topology)?;
        let mut previous_service: Option<&FleetServiceId> = None;

        for target in &self.targets {
            if previous_service.is_some_and(|previous| previous >= &target.service) {
                return Err(FleetServiceTopologyError::NonCanonicalTargetOrder {
                    service: target.service.clone(),
                });
            }
            previous_service = Some(&target.service);
            let target_occurrences = occurrences.remove(&target.service).ok_or_else(|| {
                FleetServiceTopologyError::OrphanServiceTarget {
                    service: target.service.clone(),
                }
            })?;
            validate_target(target, &target_occurrences, component_topology)?;
        }

        if let Some((service, occurrence)) =
            occurrences.into_iter().find_map(|(service, occurrences)| {
                occurrences.into_iter().next().map(|one| (service, one))
            })
        {
            return Err(FleetServiceTopologyError::OrphanServiceOccurrence {
                service,
                deployment: occurrence.deployment,
                member_path: occurrence.member_path,
            });
        }
        Ok(())
    }
}

/// One canonical logical Fleet-service target.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServiceTarget {
    pub service: FleetServiceId,
    pub role: CanisterRole,
    pub component_spec: ComponentSpecId,
    pub mode: FleetServiceTargetMode,
    pub placement: FleetServicePlacementPolicy,
}

/// Mode-specific target contract before concrete service members exist.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetServiceTargetMode {
    AuthorityReplica {
        authority_deployment: ComponentGroupDeploymentId,
        authority_member: ComponentGroupMemberPath,
    },
    ActivePool,
}

/// Service-wide density and spread envelope independent of deployment placement.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServicePlacementPolicy {
    pub maximum_members_per_root: u32,
    pub minimum_distinct_roots: u32,
}

/// Typed rejection for an invalid Fleet-service target topology.
#[derive(Debug, ThisError)]
pub enum FleetServiceTopologyError {
    #[error(transparent)]
    ComponentGroupDeploymentTopology(#[from] ComponentGroupDeploymentTopologyError),

    #[error(transparent)]
    ComponentTopology(#[from] ComponentTopologyError),

    #[error("canonical Fleet-service topology bytes {actual} exceed bound {maximum}")]
    CanonicalBytesBoundExceeded { actual: usize, maximum: usize },

    #[error("Fleet-service target count {actual} exceeds bound {maximum}")]
    TargetBoundExceeded { actual: usize, maximum: usize },

    #[error("Fleet-service target '{service}' is not in canonical order")]
    NonCanonicalTargetOrder { service: FleetServiceId },

    #[error("Fleet-service target projection does not match checked-in target declarations")]
    TargetProjectionMismatch,

    #[error("Fleet-service target '{service}' has no deployment occurrence")]
    OrphanServiceTarget { service: FleetServiceId },

    #[error(
        "Fleet-service occurrence '{service}' at deployment '{deployment}' member '{member_path:?}' has no target"
    )]
    OrphanServiceOccurrence {
        service: FleetServiceId,
        deployment: ComponentGroupDeploymentId,
        member_path: ComponentGroupMemberPath,
    },

    #[error(
        "Fleet-service target '{service}' references unknown Component Spec '{component_spec}'"
    )]
    UnknownTargetComponentSpec {
        service: FleetServiceId,
        component_spec: ComponentSpecId,
    },

    #[error(
        "Fleet-service target '{service}' role '{received}' does not match Component Spec role '{expected}'"
    )]
    TargetRoleMismatch {
        service: FleetServiceId,
        expected: CanisterRole,
        received: CanisterRole,
    },

    #[error(
        "Fleet-service occurrence '{service}' uses Component Spec '{received}' instead of target Spec '{expected}'"
    )]
    OccurrenceComponentSpecMismatch {
        service: FleetServiceId,
        expected: ComponentSpecId,
        received: ComponentSpecId,
    },

    #[error(
        "Fleet-service Authority deployment '{deployment}' must have exactly one initial and maximum placement"
    )]
    AuthorityDeploymentPlacementCountInvalid {
        deployment: ComponentGroupDeploymentId,
        initial_placements: u32,
        maximum_placements: u32,
    },

    #[error("AuthorityReplica service '{service}' has no Authority occurrence")]
    MissingServiceAuthority { service: FleetServiceId },

    #[error("AuthorityReplica service '{service}' has {actual} Authority occurrences")]
    DuplicateServiceAuthority {
        service: FleetServiceId,
        actual: usize,
    },

    #[error(
        "AuthorityReplica service '{service}' selector does not name its exact Authority occurrence"
    )]
    AuthoritySelectorMismatch { service: FleetServiceId },

    #[error("AuthorityReplica service '{service}' contains a PoolMember occurrence")]
    AuthorityReplicaContainsPoolMember { service: FleetServiceId },

    #[error("ActivePool service '{service}' contains an Authority or Replica occurrence")]
    ActivePoolContainsNonPoolMember { service: FleetServiceId },

    #[error("ActivePool service '{service}' has no initially materialized PoolMember")]
    ActivePoolHasNoInitialMember { service: FleetServiceId },

    #[error("Fleet-service member count overflowed for '{service}'")]
    ServiceMemberCountOverflow { service: FleetServiceId },

    #[error("Fleet-service target '{service}' has zero maximum members per root")]
    ZeroMaximumMembersPerRoot { service: FleetServiceId },

    #[error(
        "Fleet-service target '{service}' maximum members per root {maximum_members_per_root} exceed maximum concurrent members {maximum_members}"
    )]
    MaximumMembersPerRootExceedsMaximum {
        service: FleetServiceId,
        maximum_members_per_root: u32,
        maximum_members: u32,
    },

    #[error(
        "Fleet-service target '{service}' maximum members per root {maximum_members_per_root} cannot fit {required_members_per_root} members from one placement of deployment '{deployment}'"
    )]
    MaximumMembersPerRootBelowPlacementWidth {
        service: FleetServiceId,
        deployment: ComponentGroupDeploymentId,
        maximum_members_per_root: u32,
        required_members_per_root: u32,
    },

    #[error("Fleet-service target '{service}' has zero minimum distinct roots")]
    ZeroMinimumDistinctRoots { service: FleetServiceId },

    #[error(
        "Fleet-service target '{service}' minimum distinct roots {minimum_distinct_roots} exceed maximum concurrent members {maximum_members}"
    )]
    MinimumDistinctRootsExceedsMaximum {
        service: FleetServiceId,
        minimum_distinct_roots: u32,
        maximum_members: u32,
    },

    #[error(
        "Fleet-service target '{service}' minimum distinct roots {minimum_distinct_roots} exceed maximum contributing placements {maximum_placements}"
    )]
    MinimumDistinctRootsExceedsMaximumPlacements {
        service: FleetServiceId,
        minimum_distinct_roots: u32,
        maximum_placements: u32,
    },
}

struct FleetServiceOccurrence {
    deployment: ComponentGroupDeploymentId,
    member_path: ComponentGroupMemberPath,
    component_spec: ComponentSpecId,
    purpose: FleetServiceMemberPurpose,
    initial_placements: u32,
    maximum_placements: u32,
}

#[derive(Default)]
struct FleetServiceDemand {
    initial_members: u32,
    maximum_members: u32,
    deployments: BTreeMap<ComponentGroupDeploymentId, FleetServiceDeploymentDemand>,
}

struct FleetServiceDeploymentDemand {
    member_width: u32,
    maximum_placements: u32,
}

impl FleetServiceDemand {
    fn observe(
        &mut self,
        target: &FleetServiceTarget,
        occurrence: &FleetServiceOccurrence,
    ) -> Result<(), FleetServiceTopologyError> {
        self.initial_members = self
            .initial_members
            .checked_add(occurrence.initial_placements)
            .ok_or_else(|| service_member_count_overflow(target))?;
        self.maximum_members = self
            .maximum_members
            .checked_add(occurrence.maximum_placements)
            .ok_or_else(|| service_member_count_overflow(target))?;
        let deployment = self
            .deployments
            .entry(occurrence.deployment.clone())
            .or_insert(FleetServiceDeploymentDemand {
                member_width: 0,
                maximum_placements: occurrence.maximum_placements,
            });
        deployment.member_width = deployment
            .member_width
            .checked_add(1)
            .ok_or_else(|| service_member_count_overflow(target))?;
        Ok(())
    }

    fn maximum_contributing_placements(
        &self,
        target: &FleetServiceTarget,
    ) -> Result<u32, FleetServiceTopologyError> {
        self.deployments.values().try_fold(0_u32, |total, demand| {
            total
                .checked_add(demand.maximum_placements)
                .ok_or_else(|| service_member_count_overflow(target))
        })
    }

    fn widest_placement(
        &self,
    ) -> Option<(&ComponentGroupDeploymentId, &FleetServiceDeploymentDemand)> {
        self.deployments
            .iter()
            .max_by_key(|(_deployment, demand)| demand.member_width)
    }
}

fn compile_target(
    service: &FleetServiceId,
    source: &FleetServiceTargetConfig,
) -> FleetServiceTarget {
    match source {
        FleetServiceTargetConfig::AuthorityReplica {
            role,
            component_spec,
            authority_deployment,
            authority_member,
            placement,
        } => FleetServiceTarget {
            service: service.clone(),
            role: role.clone(),
            component_spec: component_spec.clone(),
            mode: FleetServiceTargetMode::AuthorityReplica {
                authority_deployment: authority_deployment.clone(),
                authority_member: authority_member.clone(),
            },
            placement: compile_placement(placement),
        },
        FleetServiceTargetConfig::ActivePool {
            role,
            component_spec,
            placement,
        } => FleetServiceTarget {
            service: service.clone(),
            role: role.clone(),
            component_spec: component_spec.clone(),
            mode: FleetServiceTargetMode::ActivePool,
            placement: compile_placement(placement),
        },
    }
}

const fn compile_placement(
    source: &FleetServicePlacementPolicyConfig,
) -> FleetServicePlacementPolicy {
    FleetServicePlacementPolicy {
        maximum_members_per_root: source.maximum_members_per_root,
        minimum_distinct_roots: source.minimum_distinct_roots,
    }
}

const fn validate_target_count(count: usize) -> Result<(), FleetServiceTopologyError> {
    if count > MAX_FLEET_SERVICE_TARGETS {
        return Err(FleetServiceTopologyError::TargetBoundExceeded {
            actual: count,
            maximum: MAX_FLEET_SERVICE_TARGETS,
        });
    }
    Ok(())
}

fn service_occurrences(
    deployment_topology: &ComponentGroupDeploymentTopology,
) -> Result<BTreeMap<FleetServiceId, Vec<FleetServiceOccurrence>>, FleetServiceTopologyError> {
    let mut services: BTreeMap<FleetServiceId, Vec<FleetServiceOccurrence>> = BTreeMap::new();
    for deployment in &deployment_topology.component_group_deployments {
        let has_authority = deployment.members.iter().any(|member| {
            matches!(
                member.purpose,
                ComponentDeploymentPurpose::FleetServiceMember {
                    member_purpose: FleetServiceMemberPurpose::Authority,
                    ..
                }
            )
        });
        if has_authority
            && (deployment.initial_placements != 1 || deployment.maximum_placements != 1)
        {
            return Err(
                FleetServiceTopologyError::AuthorityDeploymentPlacementCountInvalid {
                    deployment: deployment.deployment.clone(),
                    initial_placements: deployment.initial_placements,
                    maximum_placements: deployment.maximum_placements,
                },
            );
        }
        for member in &deployment.members {
            let ComponentDeploymentPurpose::FleetServiceMember {
                service,
                member_purpose,
            } = &member.purpose
            else {
                continue;
            };
            services
                .entry(service.clone())
                .or_default()
                .push(FleetServiceOccurrence {
                    deployment: deployment.deployment.clone(),
                    member_path: member.member_path.clone(),
                    component_spec: member.component_spec.clone(),
                    purpose: *member_purpose,
                    initial_placements: deployment.initial_placements,
                    maximum_placements: deployment.maximum_placements,
                });
        }
    }
    Ok(services)
}

fn validate_target(
    target: &FleetServiceTarget,
    occurrences: &[FleetServiceOccurrence],
    component_topology: &ComponentTopology,
) -> Result<(), FleetServiceTopologyError> {
    let component_spec = component_topology
        .get(&target.component_spec)
        .ok_or_else(|| FleetServiceTopologyError::UnknownTargetComponentSpec {
            service: target.service.clone(),
            component_spec: target.component_spec.clone(),
        })?;
    if target.role != component_spec.component_role {
        return Err(FleetServiceTopologyError::TargetRoleMismatch {
            service: target.service.clone(),
            expected: component_spec.component_role.clone(),
            received: target.role.clone(),
        });
    }
    let mut demand = FleetServiceDemand::default();
    for occurrence in occurrences {
        if occurrence.component_spec != target.component_spec {
            return Err(FleetServiceTopologyError::OccurrenceComponentSpecMismatch {
                service: target.service.clone(),
                expected: target.component_spec.clone(),
                received: occurrence.component_spec.clone(),
            });
        }
        demand.observe(target, occurrence)?;
    }
    validate_service_placement(target, &demand)?;
    match &target.mode {
        FleetServiceTargetMode::AuthorityReplica {
            authority_deployment,
            authority_member,
        } => {
            validate_authority_replica(target, occurrences, authority_deployment, authority_member)
        }
        FleetServiceTargetMode::ActivePool => {
            validate_active_pool(target, occurrences, demand.initial_members)
        }
    }
}

fn validate_service_placement(
    target: &FleetServiceTarget,
    demand: &FleetServiceDemand,
) -> Result<(), FleetServiceTopologyError> {
    if target.placement.maximum_members_per_root == 0 {
        return Err(FleetServiceTopologyError::ZeroMaximumMembersPerRoot {
            service: target.service.clone(),
        });
    }
    if target.placement.maximum_members_per_root > demand.maximum_members {
        return Err(
            FleetServiceTopologyError::MaximumMembersPerRootExceedsMaximum {
                service: target.service.clone(),
                maximum_members_per_root: target.placement.maximum_members_per_root,
                maximum_members: demand.maximum_members,
            },
        );
    }
    if let Some((deployment, placement)) = demand.widest_placement()
        && target.placement.maximum_members_per_root < placement.member_width
    {
        return Err(
            FleetServiceTopologyError::MaximumMembersPerRootBelowPlacementWidth {
                service: target.service.clone(),
                deployment: deployment.clone(),
                maximum_members_per_root: target.placement.maximum_members_per_root,
                required_members_per_root: placement.member_width,
            },
        );
    }
    if target.placement.minimum_distinct_roots == 0 {
        return Err(FleetServiceTopologyError::ZeroMinimumDistinctRoots {
            service: target.service.clone(),
        });
    }
    if target.placement.minimum_distinct_roots > demand.maximum_members {
        return Err(
            FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximum {
                service: target.service.clone(),
                minimum_distinct_roots: target.placement.minimum_distinct_roots,
                maximum_members: demand.maximum_members,
            },
        );
    }
    let maximum_placements = demand.maximum_contributing_placements(target)?;
    if target.placement.minimum_distinct_roots > maximum_placements {
        return Err(
            FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximumPlacements {
                service: target.service.clone(),
                minimum_distinct_roots: target.placement.minimum_distinct_roots,
                maximum_placements,
            },
        );
    }
    Ok(())
}

fn service_member_count_overflow(target: &FleetServiceTarget) -> FleetServiceTopologyError {
    FleetServiceTopologyError::ServiceMemberCountOverflow {
        service: target.service.clone(),
    }
}

fn validate_authority_replica(
    target: &FleetServiceTarget,
    occurrences: &[FleetServiceOccurrence],
    authority_deployment: &ComponentGroupDeploymentId,
    authority_member: &ComponentGroupMemberPath,
) -> Result<(), FleetServiceTopologyError> {
    if occurrences
        .iter()
        .any(|occurrence| occurrence.purpose == FleetServiceMemberPurpose::PoolMember)
    {
        return Err(
            FleetServiceTopologyError::AuthorityReplicaContainsPoolMember {
                service: target.service.clone(),
            },
        );
    }
    let authorities = occurrences
        .iter()
        .filter(|occurrence| occurrence.purpose == FleetServiceMemberPurpose::Authority)
        .collect::<Vec<_>>();
    if authorities.is_empty() {
        return Err(FleetServiceTopologyError::MissingServiceAuthority {
            service: target.service.clone(),
        });
    }
    if authorities.len() > 1 {
        return Err(FleetServiceTopologyError::DuplicateServiceAuthority {
            service: target.service.clone(),
            actual: authorities.len(),
        });
    }
    let authority = authorities[0];
    if authority.deployment != *authority_deployment || authority.member_path != *authority_member {
        return Err(FleetServiceTopologyError::AuthoritySelectorMismatch {
            service: target.service.clone(),
        });
    }
    Ok(())
}

fn validate_active_pool(
    target: &FleetServiceTarget,
    occurrences: &[FleetServiceOccurrence],
    initial_members: u32,
) -> Result<(), FleetServiceTopologyError> {
    if occurrences
        .iter()
        .any(|occurrence| occurrence.purpose != FleetServiceMemberPurpose::PoolMember)
    {
        return Err(FleetServiceTopologyError::ActivePoolContainsNonPoolMember {
            service: target.service.clone(),
        });
    }
    if initial_members == 0 {
        return Err(FleetServiceTopologyError::ActivePoolHasNoInitialMember {
            service: target.service.clone(),
        });
    }
    Ok(())
}
