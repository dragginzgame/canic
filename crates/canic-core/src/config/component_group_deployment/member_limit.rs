//! Module: config::component_group_deployment::member_limit
//!
//! Responsibility: compile reduction-only limits for exact flattened deployment members.
//! Does not own: Component Spec envelopes, placement, persistence, or runtime enforcement.
//! Boundary: source reductions become canonical declarations and effective member quotas.

use crate::{
    config::{
        ComponentSpec, ComponentTopology, FlattenedComponentGroupMember,
        schema::{ComponentChildKind, ComponentDeploymentMemberLimitConfig as SourceMemberLimit},
    },
    ids::{CanisterRole, ComponentGroupDeploymentId, ComponentGroupMemberPath, ComponentSpecId},
};
use std::collections::{BTreeMap, BTreeSet};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

/// Maximum member-limit declarations accepted by one deployment.
pub const MAX_COMPONENT_DEPLOYMENT_MEMBER_LIMITS: usize = 4_096;
/// Maximum total spawn-grant reductions accepted by one deployment.
pub const MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS: usize = 4_096;

/// Canonical reduction-only declaration for one exact flattened member.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentMemberLimit {
    pub member: ComponentGroupMemberPath,
    pub maximum_descendants: Option<u32>,
    pub maximum_registry_bytes: Option<u64>,
    pub spawn_grants: Vec<ComponentDeploymentSpawnGrantLimit>,
}

/// Canonical reduced ceiling for one exact Component Spec spawn grant.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentSpawnGrantLimit {
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub maximum_instances_per_parent: u32,
}

/// Fully effective quotas inherited by every placement of one member occurrence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentLimits {
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub spawn_grant_reductions: Vec<ComponentDeploymentSpawnGrantLimit>,
}

/// Typed rejection for an invalid deployment-member reduction.
#[derive(Debug, ThisError)]
pub enum ComponentDeploymentMemberLimitError {
    #[error(
        "Component Group deployment '{deployment}' member-limit count {actual} exceeds bound {maximum}"
    )]
    MemberLimitBoundExceeded {
        deployment: ComponentGroupDeploymentId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Group deployment '{deployment}' spawn-grant reduction count {actual} exceeds bound {maximum}"
    )]
    SpawnGrantReductionBoundExceeded {
        deployment: ComponentGroupDeploymentId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Group deployment '{deployment}' member-limit path '{member:?}' is absent or ambiguous"
    )]
    UnknownMemberLimitPath {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },

    #[error(
        "Component Group deployment '{deployment}' member-limit path '{member:?}' references unknown Component Spec '{component_spec}'"
    )]
    UnknownComponentSpec {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        component_spec: ComponentSpecId,
    },

    #[error("Component Group deployment '{deployment}' repeats member-limit path '{member:?}'")]
    DuplicateMemberLimitPath {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },

    #[error(
        "Component Group deployment '{deployment}' member-limit path '{member:?}' is not in canonical order"
    )]
    NonCanonicalMemberLimitOrder {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' has noncanonical limit reductions"
    )]
    NonCanonicalMemberLimitProjection {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },

    #[error("Component Group deployment '{deployment}' member '{member:?}' has zero {limit_name}")]
    ZeroAggregateLimit {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        limit_name: &'static str,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' {limit_name} {requested} exceeds Component Spec '{component_spec}' ceiling {maximum}"
    )]
    AggregateLimitExceedsSpec {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        component_spec: ComponentSpecId,
        limit_name: &'static str,
        requested: u64,
        maximum: u64,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' repeats spawn-grant reduction '{parent_role}' -> '{child_role}'"
    )]
    DuplicateSpawnGrantLimit {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' references unknown spawn grant '{parent_role}' -> '{child_role}'"
    )]
    UnknownSpawnGrant {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' has zero limit for spawn grant '{parent_role}' -> '{child_role}'"
    )]
    ZeroSpawnGrantLimit {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' changes Singleton spawn grant '{parent_role}' -> '{child_role}' from exactly one"
    )]
    InvalidSingletonSpawnGrantLimit {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' spawn-grant limit {requested} for '{parent_role}' -> '{child_role}' exceeds Component Spec ceiling {maximum}"
    )]
    SpawnGrantLimitExceedsSpec {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
        parent_role: CanisterRole,
        child_role: CanisterRole,
        requested: u32,
        maximum: u32,
    },

    #[error(
        "Component Group deployment '{deployment}' member '{member:?}' has mismatched effective limits"
    )]
    EffectiveLimitProjectionMismatch {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },
}

pub(super) fn compile_member_limits(
    deployment: &ComponentGroupDeploymentId,
    source: &[SourceMemberLimit],
    members: &[FlattenedComponentGroupMember],
    component_topology: &ComponentTopology,
) -> Result<Vec<ComponentDeploymentMemberLimit>, ComponentDeploymentMemberLimitError> {
    let requested = source
        .iter()
        .map(|limit| ComponentDeploymentMemberLimit {
            member: limit.member.clone(),
            maximum_descendants: limit.maximum_descendants,
            maximum_registry_bytes: limit.maximum_registry_bytes,
            spawn_grants: limit
                .spawn_grants
                .iter()
                .map(|grant| ComponentDeploymentSpawnGrantLimit {
                    parent_role: grant.parent_role.clone(),
                    child_role: grant.child_role.clone(),
                    maximum_instances_per_parent: grant.maximum_instances_per_parent,
                })
                .collect(),
        })
        .collect();
    canonicalize_member_limits(deployment, requested, members, component_topology)
}

pub(super) fn validate_member_limits(
    deployment: &ComponentGroupDeploymentId,
    member_limits: &[ComponentDeploymentMemberLimit],
    members: &[FlattenedComponentGroupMember],
    component_topology: &ComponentTopology,
) -> Result<(), ComponentDeploymentMemberLimitError> {
    let mut previous: Option<&ComponentGroupMemberPath> = None;
    for limit in member_limits {
        if previous.is_some_and(|member| member >= &limit.member) {
            return Err(
                ComponentDeploymentMemberLimitError::NonCanonicalMemberLimitOrder {
                    deployment: deployment.clone(),
                    member: limit.member.clone(),
                },
            );
        }
        previous = Some(&limit.member);
    }
    let canonical = canonicalize_member_limits(
        deployment,
        member_limits.to_vec(),
        members,
        component_topology,
    )?;
    if canonical != member_limits {
        let member = canonical
            .iter()
            .zip(member_limits)
            .find_map(|(expected, actual)| (expected != actual).then(|| actual.member.clone()))
            .or_else(|| member_limits.last().map(|limit| limit.member.clone()))
            .or_else(|| canonical.last().map(|limit| limit.member.clone()))
            .expect("unequal member-limit projections contain an entry");
        return Err(
            ComponentDeploymentMemberLimitError::NonCanonicalMemberLimitProjection {
                deployment: deployment.clone(),
                member,
            },
        );
    }
    Ok(())
}

pub(super) fn effective_limits(
    spec: &ComponentSpec,
    member: &ComponentGroupMemberPath,
    member_limits: &[ComponentDeploymentMemberLimit],
) -> ComponentDeploymentLimits {
    let reduction = member_limits
        .binary_search_by(|candidate| candidate.member.cmp(member))
        .ok()
        .map(|index| &member_limits[index]);
    ComponentDeploymentLimits {
        maximum_descendants: reduction
            .and_then(|limit| limit.maximum_descendants)
            .unwrap_or(spec.limits.maximum_descendants),
        maximum_registry_bytes: reduction
            .and_then(|limit| limit.maximum_registry_bytes)
            .unwrap_or(spec.limits.maximum_registry_bytes),
        spawn_grant_reductions: reduction
            .map(|limit| limit.spawn_grants.clone())
            .unwrap_or_default(),
    }
}

fn canonicalize_member_limits(
    deployment: &ComponentGroupDeploymentId,
    requested: Vec<ComponentDeploymentMemberLimit>,
    members: &[FlattenedComponentGroupMember],
    component_topology: &ComponentTopology,
) -> Result<Vec<ComponentDeploymentMemberLimit>, ComponentDeploymentMemberLimitError> {
    validate_bounds(deployment, &requested)?;
    let member_specs = member_spec_index(deployment, members)?;
    let mut seen = BTreeSet::new();
    let mut canonical = Vec::with_capacity(requested.len());
    for limit in requested {
        if !seen.insert(limit.member.clone()) {
            return Err(
                ComponentDeploymentMemberLimitError::DuplicateMemberLimitPath {
                    deployment: deployment.clone(),
                    member: limit.member,
                },
            );
        }
        let component_spec = member_specs.get(&limit.member).ok_or_else(|| {
            ComponentDeploymentMemberLimitError::UnknownMemberLimitPath {
                deployment: deployment.clone(),
                member: limit.member.clone(),
            }
        })?;
        let spec = component_topology.get(component_spec).ok_or_else(|| {
            ComponentDeploymentMemberLimitError::UnknownComponentSpec {
                deployment: deployment.clone(),
                member: limit.member.clone(),
                component_spec: component_spec.clone(),
            }
        })?;
        if let Some(limit) = canonicalize_member_limit(deployment, limit, spec)? {
            canonical.push(limit);
        }
    }
    canonical.sort_by(|left, right| left.member.cmp(&right.member));
    Ok(canonical)
}

fn validate_bounds(
    deployment: &ComponentGroupDeploymentId,
    requested: &[ComponentDeploymentMemberLimit],
) -> Result<(), ComponentDeploymentMemberLimitError> {
    if requested.len() > MAX_COMPONENT_DEPLOYMENT_MEMBER_LIMITS {
        return Err(
            ComponentDeploymentMemberLimitError::MemberLimitBoundExceeded {
                deployment: deployment.clone(),
                actual: requested.len(),
                maximum: MAX_COMPONENT_DEPLOYMENT_MEMBER_LIMITS,
            },
        );
    }
    let reduction_count = requested.iter().try_fold(0_usize, |count, limit| {
        count.checked_add(limit.spawn_grants.len()).ok_or_else(|| {
            ComponentDeploymentMemberLimitError::SpawnGrantReductionBoundExceeded {
                deployment: deployment.clone(),
                actual: usize::MAX,
                maximum: MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS,
            }
        })
    })?;
    if reduction_count > MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS {
        return Err(
            ComponentDeploymentMemberLimitError::SpawnGrantReductionBoundExceeded {
                deployment: deployment.clone(),
                actual: reduction_count,
                maximum: MAX_COMPONENT_DEPLOYMENT_SPAWN_GRANT_REDUCTIONS,
            },
        );
    }
    Ok(())
}

fn member_spec_index(
    deployment: &ComponentGroupDeploymentId,
    members: &[FlattenedComponentGroupMember],
) -> Result<BTreeMap<ComponentGroupMemberPath, ComponentSpecId>, ComponentDeploymentMemberLimitError>
{
    let mut index = BTreeMap::new();
    for member in members {
        if index
            .insert(member.member_path.clone(), member.component_spec.clone())
            .is_some()
        {
            return Err(
                ComponentDeploymentMemberLimitError::UnknownMemberLimitPath {
                    deployment: deployment.clone(),
                    member: member.member_path.clone(),
                },
            );
        }
    }
    Ok(index)
}

fn canonicalize_member_limit(
    deployment: &ComponentGroupDeploymentId,
    mut limit: ComponentDeploymentMemberLimit,
    spec: &ComponentSpec,
) -> Result<Option<ComponentDeploymentMemberLimit>, ComponentDeploymentMemberLimitError> {
    limit.maximum_descendants = canonical_u32_reduction(
        deployment,
        &limit.member,
        &spec.component_spec,
        "maximum_descendants",
        limit.maximum_descendants,
        spec.limits.maximum_descendants,
    )?;
    limit.maximum_registry_bytes = canonical_u64_reduction(
        deployment,
        &limit.member,
        &spec.component_spec,
        "maximum_registry_bytes",
        limit.maximum_registry_bytes,
        spec.limits.maximum_registry_bytes,
    )?;
    limit.spawn_grants =
        canonical_spawn_grants(deployment, &limit.member, limit.spawn_grants, spec)?;
    if limit.maximum_descendants.is_none()
        && limit.maximum_registry_bytes.is_none()
        && limit.spawn_grants.is_empty()
    {
        return Ok(None);
    }
    Ok(Some(limit))
}

fn canonical_u32_reduction(
    deployment: &ComponentGroupDeploymentId,
    member: &ComponentGroupMemberPath,
    component_spec: &ComponentSpecId,
    limit_name: &'static str,
    requested: Option<u32>,
    maximum: u32,
) -> Result<Option<u32>, ComponentDeploymentMemberLimitError> {
    canonical_u64_reduction(
        deployment,
        member,
        component_spec,
        limit_name,
        requested.map(u64::from),
        u64::from(maximum),
    )
    .map(|value| value.map(|value| u32::try_from(value).expect("u32 reduction remains u32")))
}

fn canonical_u64_reduction(
    deployment: &ComponentGroupDeploymentId,
    member: &ComponentGroupMemberPath,
    component_spec: &ComponentSpecId,
    limit_name: &'static str,
    requested: Option<u64>,
    maximum: u64,
) -> Result<Option<u64>, ComponentDeploymentMemberLimitError> {
    let Some(requested) = requested else {
        return Ok(None);
    };
    if requested == 0 {
        return Err(ComponentDeploymentMemberLimitError::ZeroAggregateLimit {
            deployment: deployment.clone(),
            member: member.clone(),
            limit_name,
        });
    }
    if requested > maximum {
        return Err(
            ComponentDeploymentMemberLimitError::AggregateLimitExceedsSpec {
                deployment: deployment.clone(),
                member: member.clone(),
                component_spec: component_spec.clone(),
                limit_name,
                requested,
                maximum,
            },
        );
    }
    Ok((requested < maximum).then_some(requested))
}

fn canonical_spawn_grants(
    deployment: &ComponentGroupDeploymentId,
    member: &ComponentGroupMemberPath,
    requested: Vec<ComponentDeploymentSpawnGrantLimit>,
    spec: &ComponentSpec,
) -> Result<Vec<ComponentDeploymentSpawnGrantLimit>, ComponentDeploymentMemberLimitError> {
    let mut canonical = BTreeMap::new();
    for limit in requested {
        let identity = (limit.parent_role.clone(), limit.child_role.clone());
        if canonical.contains_key(&identity) {
            return Err(
                ComponentDeploymentMemberLimitError::DuplicateSpawnGrantLimit {
                    deployment: deployment.clone(),
                    member: member.clone(),
                    parent_role: limit.parent_role,
                    child_role: limit.child_role,
                },
            );
        }
        let grant = spec
            .spawn_grant(&limit.parent_role, &limit.child_role)
            .ok_or_else(|| ComponentDeploymentMemberLimitError::UnknownSpawnGrant {
                deployment: deployment.clone(),
                member: member.clone(),
                parent_role: limit.parent_role.clone(),
                child_role: limit.child_role.clone(),
            })?;
        if limit.maximum_instances_per_parent == 0 {
            return Err(ComponentDeploymentMemberLimitError::ZeroSpawnGrantLimit {
                deployment: deployment.clone(),
                member: member.clone(),
                parent_role: limit.parent_role,
                child_role: limit.child_role,
            });
        }
        if spec
            .child(&limit.child_role)
            .is_some_and(|child| child.kind == ComponentChildKind::Singleton)
            && limit.maximum_instances_per_parent != 1
        {
            return Err(
                ComponentDeploymentMemberLimitError::InvalidSingletonSpawnGrantLimit {
                    deployment: deployment.clone(),
                    member: member.clone(),
                    parent_role: limit.parent_role,
                    child_role: limit.child_role,
                },
            );
        }
        if limit.maximum_instances_per_parent > grant.maximum_instances_per_parent {
            return Err(
                ComponentDeploymentMemberLimitError::SpawnGrantLimitExceedsSpec {
                    deployment: deployment.clone(),
                    member: member.clone(),
                    parent_role: limit.parent_role,
                    child_role: limit.child_role,
                    requested: limit.maximum_instances_per_parent,
                    maximum: grant.maximum_instances_per_parent,
                },
            );
        }
        if limit.maximum_instances_per_parent < grant.maximum_instances_per_parent {
            canonical.insert(identity, limit);
        }
    }
    Ok(canonical.into_values().collect())
}
