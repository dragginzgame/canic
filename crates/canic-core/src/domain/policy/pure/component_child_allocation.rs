//! Module: domain::policy::pure::component_child_allocation
//!
//! Responsibility: decide whether one registered Component-tree node may reserve a direct child.
//! Does not own: caller lookup, Registry storage, artifact resolution, or Canister effects.
//! Boundary: workflow supplies exact active Registry evidence and durable allocation counts.

use crate::{
    cdk::types::Principal,
    config::{ComponentDeploymentLimits, ComponentTopology, schema::ComponentChildKind},
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetSubnetRootBinding, ManagedCanisterBinding,
    },
};
use thiserror::Error as ThisError;

///
/// ComponentRegistryVersionEvidence
///
/// Exact logical Component Registry head named by a child-allocation request or observation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentRegistryVersionEvidence {
    pub component: ComponentInstanceId,
    pub revision: u64,
    pub content_hash: [u8; 32],
}

///
/// ComponentChildAllocationReadiness
///
/// Exact prerequisite that prevents a direct-child reservation from proceeding.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentChildAllocationReadiness {
    Ready,
    /// The exact parent and partition are prepared but not active yet; only
    /// the compiled initial sharding/scaling allowance may be consumed.
    InitialBootstrap,
    FleetRegistryRootInactive,
    RootRuntimeInactive,
    ComponentRegistryInactive,
    ParentRegistryMemberInactive,
}

/// Lifecycle lane approved for one exact direct-child reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentChildAllocationMode {
    InitialBootstrap,
    Active,
}

///
/// ComponentChildAllocationInput
///
/// Complete pure evidence needed to authorize one direct-child reservation.
///

pub struct ComponentChildAllocationInput<'a> {
    pub operation_id: [u8; 32],
    pub caller: Principal,
    pub component: &'a ComponentBinding,
    pub parent: &'a ManagedCanisterBinding,
    pub child_role: &'a CanisterRole,
    pub expected_registry: ComponentRegistryVersionEvidence,
    pub current_registry: ComponentRegistryVersionEvidence,
    pub readiness: ComponentChildAllocationReadiness,
    pub root: &'a FleetSubnetRootBinding,
    pub topology: &'a ComponentTopology,
    /// Exact Spec-bounded limits inherited from the owning top-level Component deployment.
    pub deployment_limits: &'a ComponentDeploymentLimits,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    /// Reserved plus committed, non-removed descendants in this exact Component tree.
    pub component_descendants: u32,
    /// Reserved plus committed, non-removed descendants across this root.
    pub root_managed_descendants: u32,
    /// Reserved plus committed, non-removed children for this exact parent and role.
    pub parent_role_instances: u32,
}

///
/// ComponentChildAllocationDecision
///
/// Normalized direct-child identity and grant facts approved for durable reservation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentChildAllocationDecision {
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub parent_canister_id: Principal,
    pub parent_role: CanisterRole,
    pub child_role: CanisterRole,
    pub child_kind: ComponentChildKind,
    pub mode: ComponentChildAllocationMode,
    pub maximum_instances_per_parent: u32,
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
}

///
/// ComponentChildAllocationPolicyError
///
/// Typed rejection before any root-local child reservation mutation.
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum ComponentChildAllocationPolicyError {
    #[error("Component Child allocation operation ID must not be all zeroes")]
    EmptyOperationId,

    #[error("Component Child allocation has invalid protected Component authority")]
    InvalidComponentBinding,

    #[error("Component Child allocation has invalid protected parent authority")]
    InvalidParentBinding,

    #[error("Component Child allocation parent belongs to a different Component tree")]
    ParentComponentMismatch,

    #[error("Component Child allocation caller is not its exact registered parent")]
    ParentCallerMismatch,

    #[error("Component Child allocation requires an Active Fleet Registry root")]
    FleetRegistryRootNotActive,

    #[error("Component Child allocation requires an Active Fleet Subnet Root runtime")]
    RootRuntimeNotActive,

    #[error("Component Child allocation requires an Active Component Registry partition")]
    ComponentRegistryNotActive,

    #[error("Component Child allocation requires an Active parent Registry member")]
    ParentRegistryMemberNotActive,

    #[error("Component Child initial bootstrap allowance is absent or exhausted")]
    InitialBootstrapUnavailable,

    #[error("Component Child allocation expected Registry authority is invalid or stale")]
    ComponentRegistryAuthorityMismatch,

    #[error("Component Spec '{0}' is absent from the protected topology")]
    ComponentSpecUnknown(ComponentSpecId),

    #[error("child role '{child_role}' is not admitted by Component Spec '{component_spec}'")]
    ChildRoleNotAdmitted {
        component_spec: ComponentSpecId,
        child_role: CanisterRole,
    },

    #[error(
        "registered parent role '{parent_role}' has no spawn grant for child role '{child_role}'"
    )]
    SpawnGrantMissing {
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error("per-parent child count overflowed")]
    ParentRoleCountOverflow,

    #[error(
        "registered parent role '{parent_role}' exhausted its child role '{child_role}' capacity"
    )]
    ParentRoleCapacityExhausted {
        parent_role: CanisterRole,
        child_role: CanisterRole,
    },

    #[error("Component descendant capacity is exhausted")]
    ComponentDescendantCapacityExhausted,

    #[error("Component deployment limits exceed or contradict the protected Component Spec")]
    InvalidDeploymentLimits,

    #[error("root Component instance count overflowed")]
    ComponentCountOverflow,
}

/// Decide one exact direct-child reservation without mutation.
pub fn reserve_component_child(
    input: ComponentChildAllocationInput<'_>,
) -> Result<ComponentChildAllocationDecision, ComponentChildAllocationPolicyError> {
    if input.operation_id == [0; 32] {
        return Err(ComponentChildAllocationPolicyError::EmptyOperationId);
    }
    input
        .topology
        .validate_component_binding(input.root, input.component)
        .map_err(|_| ComponentChildAllocationPolicyError::InvalidComponentBinding)?;

    let (parent_component, parent_canister_id, parent_role) = parent_identity(&input)?;
    if parent_component != input.component {
        return Err(ComponentChildAllocationPolicyError::ParentComponentMismatch);
    }
    if parent_canister_id != input.caller {
        return Err(ComponentChildAllocationPolicyError::ParentCallerMismatch);
    }
    validate_registry_authority(&input)?;

    let spec = input
        .topology
        .get(&input.component.component_spec)
        .ok_or_else(|| {
            ComponentChildAllocationPolicyError::ComponentSpecUnknown(
                input.component.component_spec.clone(),
            )
        })?;
    let child = spec.child(input.child_role).ok_or_else(|| {
        ComponentChildAllocationPolicyError::ChildRoleNotAdmitted {
            component_spec: spec.component_spec.clone(),
            child_role: input.child_role.clone(),
        }
    })?;
    let grant = spec
        .spawn_grant(parent_role, input.child_role)
        .ok_or_else(|| ComponentChildAllocationPolicyError::SpawnGrantMissing {
            parent_role: parent_role.clone(),
            child_role: input.child_role.clone(),
        })?;
    let effective_grant_maximum = effective_grant_maximum(
        input.deployment_limits,
        parent_role,
        input.child_role,
        grant.maximum_instances_per_parent,
    )?;
    if grant.initial_instances_per_parent > effective_grant_maximum {
        return Err(ComponentChildAllocationPolicyError::InvalidDeploymentLimits);
    }
    let mode = validate_readiness(
        input.readiness,
        input.parent_role_instances,
        grant.initial_instances_per_parent,
    )?;
    let limits_are_spec_bounded = input.deployment_limits.maximum_descendants > 0
        && input.deployment_limits.maximum_descendants <= spec.limits.maximum_descendants
        && input.deployment_limits.maximum_registry_bytes > 0
        && input.deployment_limits.maximum_registry_bytes <= spec.limits.maximum_registry_bytes;
    if !limits_are_spec_bounded {
        return Err(ComponentChildAllocationPolicyError::InvalidDeploymentLimits);
    }

    validate_capacity(
        &input,
        parent_role,
        effective_grant_maximum,
        input.deployment_limits.maximum_descendants,
    )?;

    Ok(ComponentChildAllocationDecision {
        component: input.component.component,
        component_spec: input.component.component_spec.clone(),
        spec_hash: input.component.spec_hash,
        parent_canister_id,
        parent_role: parent_role.clone(),
        child_role: child.role.clone(),
        child_kind: child.kind,
        mode,
        maximum_instances_per_parent: effective_grant_maximum,
        maximum_descendants: input.deployment_limits.maximum_descendants,
        maximum_registry_bytes: input.deployment_limits.maximum_registry_bytes,
    })
}

fn effective_grant_maximum(
    limits: &ComponentDeploymentLimits,
    parent_role: &CanisterRole,
    child_role: &CanisterRole,
    spec_maximum: u32,
) -> Result<u32, ComponentChildAllocationPolicyError> {
    let reduced = limits
        .spawn_grant_reductions
        .iter()
        .find(|limit| &limit.parent_role == parent_role && &limit.child_role == child_role)
        .map_or(spec_maximum, |limit| limit.maximum_instances_per_parent);
    if reduced == 0 || reduced > spec_maximum {
        return Err(ComponentChildAllocationPolicyError::InvalidDeploymentLimits);
    }
    Ok(reduced)
}

fn parent_identity<'a>(
    input: &'a ComponentChildAllocationInput<'a>,
) -> Result<(&'a ComponentBinding, Principal, &'a CanisterRole), ComponentChildAllocationPolicyError>
{
    match input.parent {
        ManagedCanisterBinding::Component(parent) => {
            input
                .topology
                .validate_component_binding(input.root, parent)
                .map_err(|_| ComponentChildAllocationPolicyError::InvalidParentBinding)?;
            Ok((parent, parent.canister_id, &parent.role))
        }
        ManagedCanisterBinding::ComponentChild(parent) => {
            input
                .topology
                .validate_component_child_binding(input.root, parent)
                .map_err(|_| ComponentChildAllocationPolicyError::InvalidParentBinding)?;
            Ok((&parent.component, parent.canister_id, &parent.role))
        }
    }
}

const fn validate_readiness(
    readiness: ComponentChildAllocationReadiness,
    parent_role_instances: u32,
    initial_instances_per_parent: u32,
) -> Result<ComponentChildAllocationMode, ComponentChildAllocationPolicyError> {
    match readiness {
        ComponentChildAllocationReadiness::Ready => Ok(ComponentChildAllocationMode::Active),
        ComponentChildAllocationReadiness::InitialBootstrap
            if initial_instances_per_parent > 0
                && parent_role_instances < initial_instances_per_parent =>
        {
            Ok(ComponentChildAllocationMode::InitialBootstrap)
        }
        ComponentChildAllocationReadiness::InitialBootstrap => {
            Err(ComponentChildAllocationPolicyError::InitialBootstrapUnavailable)
        }
        ComponentChildAllocationReadiness::FleetRegistryRootInactive => {
            Err(ComponentChildAllocationPolicyError::FleetRegistryRootNotActive)
        }
        ComponentChildAllocationReadiness::RootRuntimeInactive => {
            Err(ComponentChildAllocationPolicyError::RootRuntimeNotActive)
        }
        ComponentChildAllocationReadiness::ComponentRegistryInactive => {
            Err(ComponentChildAllocationPolicyError::ComponentRegistryNotActive)
        }
        ComponentChildAllocationReadiness::ParentRegistryMemberInactive => {
            Err(ComponentChildAllocationPolicyError::ParentRegistryMemberNotActive)
        }
    }
}

fn validate_registry_authority(
    input: &ComponentChildAllocationInput<'_>,
) -> Result<(), ComponentChildAllocationPolicyError> {
    if input.expected_registry != input.current_registry
        || input.current_registry.component != input.component.component
        || input.current_registry.revision == 0
    {
        return Err(ComponentChildAllocationPolicyError::ComponentRegistryAuthorityMismatch);
    }
    Ok(())
}

fn validate_capacity(
    input: &ComponentChildAllocationInput<'_>,
    parent_role: &CanisterRole,
    maximum_instances_per_parent: u32,
    maximum_descendants: u32,
) -> Result<(), ComponentChildAllocationPolicyError> {
    input
        .parent_role_instances
        .checked_add(1)
        .ok_or(ComponentChildAllocationPolicyError::ParentRoleCountOverflow)?;
    if input.parent_role_instances >= maximum_instances_per_parent {
        return Err(
            ComponentChildAllocationPolicyError::ParentRoleCapacityExhausted {
                parent_role: parent_role.clone(),
                child_role: input.child_role.clone(),
            },
        );
    }
    if input.component_descendants >= maximum_descendants {
        return Err(ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::{Cycles, Principal},
        config::schema::{ConfigModel, Validate},
        ids::{
            AppId, CanonicalNetworkId, ComponentChildBinding, ComponentSpecAdmission,
            CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetRootLimits, SubnetId,
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

        [roles.project_instance]
        package = "project_instance"
        kind = "canister"

        [roles.project_ledger]
        package = "project_ledger"
        kind = "canister"

        [roles.project_machine]
        package = "project_machine"
        kind = "canister"

        [component_specs.projects]
        component_role = "project_hub"
        maximum_instances = 4

        [component_specs.projects.limits]
        maximum_descendants = 20_000

        [component_specs.projects.children.project_instance]
        kind = "replica"

        [component_specs.projects.children.project_ledger]
        kind = "singleton"

        [component_specs.projects.children.project_machine]
        kind = "replica"

        [component_specs.projects.spawn_grants.project_hub.project_instance]
        maximum_instances_per_parent = 10_000

        [component_specs.projects.spawn_grants.project_instance.project_ledger]
        maximum_instances_per_parent = 1

        [component_specs.projects.spawn_grants.project_instance.project_machine]
        maximum_instances_per_parent = 4

        [component_specs.projects.scaling.pools.initial_projects]
        canister_role = "project_instance"
        policy.initial_workers = 1
        policy.min_workers = 1
        policy.max_workers = 10_000
    "#;

    #[test]
    fn exact_top_level_parent_may_reserve_only_its_direct_granted_child() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());
        let decision = reserve_component_child(input(&fixture, &parent, &child_role))
            .expect("direct child reservation");

        assert_eq!(
            decision,
            ComponentChildAllocationDecision {
                component: fixture.component.component,
                component_spec: fixture.component.component_spec.clone(),
                spec_hash: fixture.component.spec_hash,
                parent_canister_id: fixture.component.canister_id,
                parent_role: CanisterRole::new("project_hub"),
                child_role,
                child_kind: ComponentChildKind::Replica,
                mode: ComponentChildAllocationMode::Active,
                maximum_instances_per_parent: 10_000,
                maximum_descendants: 20_000,
                maximum_registry_bytes: 16_777_216,
            }
        );
    }

    #[test]
    fn registered_child_parent_uses_the_same_policy_at_arbitrary_depth() {
        let fixture = fixture();
        let parent = ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
            component: fixture.component.clone(),
            parent_canister_id: fixture.component.canister_id,
            role: CanisterRole::new("project_instance"),
            canister_id: principal(7),
        });
        let child_role = CanisterRole::new("project_ledger");
        let mut input = input(&fixture, &parent, &child_role);
        input.caller = principal(7);
        let decision =
            reserve_component_child(input).expect("grandchild reservation through same policy");

        assert_eq!(decision.parent_canister_id, principal(7));
        assert_eq!(decision.parent_role, CanisterRole::new("project_instance"));
        assert_eq!(decision.child_role, child_role);
        assert_eq!(decision.child_kind, ComponentChildKind::Singleton);
        assert_eq!(decision.maximum_instances_per_parent, 1);
    }

    #[test]
    fn catalog_presence_without_the_exact_parent_grant_never_authorizes() {
        let fixture = fixture();
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());
        let child_role = CanisterRole::new("project_ledger");

        assert_eq!(
            reserve_component_child(input(&fixture, &parent, &child_role)),
            Err(ComponentChildAllocationPolicyError::SpawnGrantMissing {
                parent_role: CanisterRole::new("project_hub"),
                child_role,
            })
        );
    }

    #[test]
    fn prepared_parent_may_consume_only_its_compiled_initial_child_allowance() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());
        let mut request = input(&fixture, &parent, &child_role);
        request.readiness = ComponentChildAllocationReadiness::InitialBootstrap;
        let decision = reserve_component_child(request).expect("compiled initial child allowance");
        assert_eq!(
            decision.mode,
            ComponentChildAllocationMode::InitialBootstrap
        );

        let mut exhausted = input(&fixture, &parent, &child_role);
        exhausted.readiness = ComponentChildAllocationReadiness::InitialBootstrap;
        exhausted.parent_role_instances = 1;
        assert_eq!(
            reserve_component_child(exhausted),
            Err(ComponentChildAllocationPolicyError::InitialBootstrapUnavailable)
        );

        let ledger_role = CanisterRole::new("project_ledger");
        let child_parent = ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
            component: fixture.component.clone(),
            parent_canister_id: fixture.component.canister_id,
            role: CanisterRole::new("project_instance"),
            canister_id: principal(7),
        });
        let mut unconfigured = input(&fixture, &child_parent, &ledger_role);
        unconfigured.caller = principal(7);
        unconfigured.readiness = ComponentChildAllocationReadiness::InitialBootstrap;
        assert_eq!(
            reserve_component_child(unconfigured),
            Err(ComponentChildAllocationPolicyError::InitialBootstrapUnavailable)
        );
    }

    #[test]
    fn caller_parent_tree_and_registry_authority_must_be_exact_and_active() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());

        let mut request = input(&fixture, &parent, &child_role);
        request.caller = principal(9);
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::ParentCallerMismatch)
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.expected_registry.content_hash = [99; 32];
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::ComponentRegistryAuthorityMismatch)
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.readiness = ComponentChildAllocationReadiness::FleetRegistryRootInactive;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::FleetRegistryRootNotActive)
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.readiness = ComponentChildAllocationReadiness::RootRuntimeInactive;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::RootRuntimeNotActive)
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.readiness = ComponentChildAllocationReadiness::ComponentRegistryInactive;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::ComponentRegistryNotActive)
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.readiness = ComponentChildAllocationReadiness::ParentRegistryMemberInactive;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::ParentRegistryMemberNotActive)
        );

        let mut other = fixture.component.clone();
        other.component = ComponentInstanceId::from_generated_bytes([88; 32]);
        let foreign_parent = ManagedCanisterBinding::Component(other);
        assert_eq!(
            reserve_component_child(input(&fixture, &foreign_parent, &child_role)),
            Err(ComponentChildAllocationPolicyError::ParentComponentMismatch)
        );
    }

    #[test]
    fn every_parent_component_and_root_capacity_is_checked_before_reservation() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());

        let mut request = input(&fixture, &parent, &child_role);
        request.parent_role_instances = 10_000;
        assert_eq!(
            reserve_component_child(request),
            Err(
                ComponentChildAllocationPolicyError::ParentRoleCapacityExhausted {
                    parent_role: CanisterRole::new("project_hub"),
                    child_role: child_role.clone(),
                }
            )
        );

        let mut request = input(&fixture, &parent, &child_role);
        request.root_managed_descendants = 19_999;
        reserve_component_child(request)
            .expect("other root descendants do not consume this Component limit");

        let mut request = input(&fixture, &parent, &child_role);
        request.component_descendants = 20_000;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted)
        );
    }

    #[test]
    fn deployment_reductions_bound_every_descendant_reservation() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());
        let reduced = ComponentDeploymentLimits {
            maximum_descendants: 2_000,
            maximum_registry_bytes: 8_388_608,
            spawn_grant_reductions: vec![crate::config::ComponentDeploymentSpawnGrantLimit {
                parent_role: CanisterRole::new("project_hub"),
                child_role: child_role.clone(),
                maximum_instances_per_parent: 3,
            }],
        };
        let mut request = input(&fixture, &parent, &child_role);
        request.deployment_limits = &reduced;
        let decision = reserve_component_child(request).expect("reduced deployment reservation");
        assert_eq!(decision.maximum_instances_per_parent, 3);
        assert_eq!(decision.maximum_descendants, 2_000);
        assert_eq!(decision.maximum_registry_bytes, 8_388_608);

        let mut parent_exhausted = input(&fixture, &parent, &child_role);
        parent_exhausted.deployment_limits = &reduced;
        parent_exhausted.parent_role_instances = 3;
        assert!(matches!(
            reserve_component_child(parent_exhausted),
            Err(ComponentChildAllocationPolicyError::ParentRoleCapacityExhausted { .. })
        ));
        let mut tree_exhausted = input(&fixture, &parent, &child_role);
        tree_exhausted.deployment_limits = &reduced;
        tree_exhausted.component_descendants = 2_000;
        assert_eq!(
            reserve_component_child(tree_exhausted),
            Err(ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted)
        );
    }

    #[test]
    fn empty_operation_and_invalid_protected_bindings_reject() {
        let fixture = fixture();
        let child_role = CanisterRole::new("project_instance");
        let parent = ManagedCanisterBinding::Component(fixture.component.clone());

        let mut request = input(&fixture, &parent, &child_role);
        request.operation_id = [0; 32];
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::EmptyOperationId)
        );

        let mut invalid_component = fixture.component.clone();
        invalid_component.spec_hash = [77; 32];
        let mut request = input(&fixture, &parent, &child_role);
        request.component = &invalid_component;
        assert_eq!(
            reserve_component_child(request),
            Err(ComponentChildAllocationPolicyError::InvalidComponentBinding)
        );

        let mut invalid_parent = fixture.component.clone();
        invalid_parent.canister_id = Principal::anonymous();
        let invalid_parent = ManagedCanisterBinding::Component(invalid_parent);
        assert_eq!(
            reserve_component_child(input(&fixture, &invalid_parent, &child_role)),
            Err(ComponentChildAllocationPolicyError::InvalidParentBinding)
        );
    }

    struct Fixture {
        topology: ComponentTopology,
        root: FleetSubnetRootBinding,
        component: ComponentBinding,
        registry: ComponentRegistryVersionEvidence,
        limits: ComponentDeploymentLimits,
    }

    fn fixture() -> Fixture {
        let config: ConfigModel = toml::from_str(CONFIG).expect("parse fixture");
        config.validate().expect("validate fixture");
        let topology = config
            .compile_component_topology()
            .expect("compile topology");
        let spec = topology
            .get(&"projects".parse().expect("Component Spec"))
            .expect("projects Spec");
        let admission = ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: 4,
        };
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
                    coordinator_subnet: subnet(2),
                    coordinator: principal(3),
                },
                epoch: 1,
            },
            placement_subnet: subnet(4),
            fleet_subnet_root: principal(5),
            component_admissions: vec![admission],
            component_topology_digest: topology
                .project_for_admissions(&[ComponentSpecAdmission {
                    component_spec: spec.component_spec.clone(),
                    spec_hash: spec.spec_hash,
                    maximum_root_instances: 4,
                }])
                .expect("project root")
                .digest()
                .expect("root digest"),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 4,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 268_435_456,
                maximum_group_placements: 16,
                canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(5_000_000_000_000),
                    creation_execution_margin: Cycles::new(1_000_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(2_000_000_000_000),
                },
            },
            funding: crate::test::support::fleet_subnet_root_funding_authority(),
        };
        let component = ComponentBinding {
            authority: root.authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([6; 32]),
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            role: spec.component_role.clone(),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: principal(6),
        };
        let registry = ComponentRegistryVersionEvidence {
            component: component.component,
            revision: 2,
            content_hash: [7; 32],
        };
        let limits = ComponentDeploymentLimits {
            maximum_descendants: spec.limits.maximum_descendants,
            maximum_registry_bytes: spec.limits.maximum_registry_bytes,
            spawn_grant_reductions: Vec::new(),
        };
        Fixture {
            topology,
            root,
            component,
            registry,
            limits,
        }
    }

    fn input<'a>(
        fixture: &'a Fixture,
        parent: &'a ManagedCanisterBinding,
        child_role: &'a CanisterRole,
    ) -> ComponentChildAllocationInput<'a> {
        ComponentChildAllocationInput {
            operation_id: [8; 32],
            caller: fixture.component.canister_id,
            component: &fixture.component,
            parent,
            child_role,
            expected_registry: fixture.registry,
            current_registry: fixture.registry,
            readiness: ComponentChildAllocationReadiness::Ready,
            root: &fixture.root,
            topology: &fixture.topology,
            deployment_limits: &fixture.limits,
            reserved_component_instances: 0,
            committed_component_instances: 1,
            component_descendants: 0,
            root_managed_descendants: 0,
            parent_role_instances: 0,
        }
    }

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    fn subnet(byte: u8) -> SubnetId {
        SubnetId::from_principal(principal(byte))
    }
}
