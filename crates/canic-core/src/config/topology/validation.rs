//! Module: config::topology::validation
//!
//! Responsibility: validate protected root, Component, and multi-level child bindings.
//! Does not own: binding construction, Registry persistence, authorization, or allocation.
//! Boundary: compares passive binding facts against one canonical compiled topology.

use crate::{
    config::{ComponentTopology, ComponentTopologyError},
    ids::{
        ComponentBinding, ComponentChildBinding, ComponentSpecAdmission, ComponentTopologyDigest,
        FleetSubnetRootBinding, FleetSubnetRootLimits,
    },
};
use candid::Principal;
use std::collections::BTreeSet;

impl ComponentTopology {
    /// Validate one root binding and return its exact canonical topology projection.
    pub fn validate_root_binding(
        &self,
        binding: &FleetSubnetRootBinding,
    ) -> Result<Self, ComponentTopologyError> {
        require_nonanonymous(
            binding.authority.binding.coordinator_subnet.as_principal(),
            "authority.binding.coordinator_subnet",
        )?;
        require_nonanonymous(
            &binding.authority.binding.coordinator,
            "authority.binding.coordinator",
        )?;
        require_nonanonymous(binding.placement_subnet.as_principal(), "placement_subnet")?;
        require_nonanonymous(&binding.fleet_subnet_root, "fleet_subnet_root")?;
        if binding.fleet_subnet_root == binding.authority.binding.coordinator {
            return Err(ComponentTopologyError::RootPrincipalConflictsWithCoordinator);
        }

        self.validate_planned_root(
            &binding.component_admissions,
            binding.component_topology_digest,
            &binding.limits,
        )
    }

    /// Validate one root's complete pre-creation topology, admission, and limit plan.
    pub fn validate_planned_root(
        &self,
        admissions: &[ComponentSpecAdmission],
        component_topology_digest: ComponentTopologyDigest,
        limits: &FleetSubnetRootLimits,
    ) -> Result<Self, ComponentTopologyError> {
        validate_root_limits(limits)?;
        let projection = self.project_for_admissions(admissions)?;
        let expected = projection.digest()?;
        if component_topology_digest != expected {
            return Err(ComponentTopologyError::RootTopologyDigestMismatch {
                expected,
                received: component_topology_digest,
            });
        }

        Ok(projection)
    }

    /// Validate every root in one Fleet plan, including uniqueness and admission sums.
    pub fn validate_fleet_subnet_root_bindings(
        &self,
        bindings: &[FleetSubnetRootBinding],
    ) -> Result<(), ComponentTopologyError> {
        let mut root_principals = BTreeSet::new();
        let mut placement_subnets = BTreeSet::new();
        let authority = bindings.first().map(|binding| &binding.authority);

        for binding in bindings {
            if authority.is_some_and(|authority| authority != &binding.authority) {
                return Err(ComponentTopologyError::RootAuthorityMismatch);
            }
            if !root_principals.insert(binding.fleet_subnet_root) {
                return Err(ComponentTopologyError::DuplicateFleetSubnetRootPrincipal {
                    fleet_subnet_root: binding.fleet_subnet_root,
                });
            }
            if !placement_subnets.insert(binding.placement_subnet) {
                return Err(ComponentTopologyError::DuplicateFleetSubnetRootSubnet {
                    placement_subnet: binding.placement_subnet,
                });
            }
            self.validate_root_binding(binding)?;
        }

        let admissions = bindings
            .iter()
            .map(|binding| binding.component_admissions.as_slice())
            .collect::<Vec<_>>();
        self.validate_fleet_admissions(&admissions)
    }

    /// Validate one concrete Component binding against its root admission and Spec.
    pub fn validate_component_binding(
        &self,
        root: &FleetSubnetRootBinding,
        binding: &ComponentBinding,
    ) -> Result<(), ComponentTopologyError> {
        self.validate_root_binding(root)?;
        require_nonanonymous(&binding.canister_id, "component.canister_id")?;

        if binding.authority != root.authority {
            return Err(ComponentTopologyError::BindingAuthorityMismatch);
        }
        if binding.placement_subnet != root.placement_subnet {
            return Err(ComponentTopologyError::BindingPlacementSubnetMismatch);
        }
        if binding.fleet_subnet_root != root.fleet_subnet_root {
            return Err(ComponentTopologyError::BindingRootMismatch);
        }
        if binding.canister_id == root.fleet_subnet_root
            || binding.canister_id == root.authority.binding.coordinator
        {
            return Err(ComponentTopologyError::ComponentPrincipalConflictsWithAuthority);
        }

        let component_spec = self.get(&binding.component_spec).ok_or_else(|| {
            ComponentTopologyError::UnknownAdmissionSpec {
                component_spec: binding.component_spec.clone(),
            }
        })?;
        let admission = find_admission(&root.component_admissions, &binding.component_spec)
            .ok_or_else(|| ComponentTopologyError::UnknownAdmissionSpec {
                component_spec: binding.component_spec.clone(),
            })?;
        if binding.spec_hash != component_spec.spec_hash || binding.spec_hash != admission.spec_hash
        {
            return Err(ComponentTopologyError::BindingSpecHashMismatch {
                component_spec: binding.component_spec.clone(),
            });
        }
        if binding.role != component_spec.component_role {
            return Err(ComponentTopologyError::BindingComponentRoleMismatch {
                component_spec: binding.component_spec.clone(),
                expected: component_spec.component_role.clone(),
                received: binding.role.clone(),
            });
        }

        Ok(())
    }

    /// Validate one child binding against its Component tree and immediate parent.
    pub fn validate_component_child_binding(
        &self,
        root: &FleetSubnetRootBinding,
        binding: &ComponentChildBinding,
    ) -> Result<(), ComponentTopologyError> {
        self.validate_component_binding(root, &binding.component)?;
        require_nonanonymous(&binding.canister_id, "component_child.canister_id")?;
        require_nonanonymous(
            &binding.parent_canister_id,
            "component_child.parent_canister_id",
        )?;

        if binding.canister_id == root.fleet_subnet_root
            || binding.canister_id == binding.component.canister_id
            || binding.canister_id == binding.parent_canister_id
            || binding.canister_id == root.authority.binding.coordinator
        {
            return Err(ComponentTopologyError::ChildPrincipalConflictsWithOwner);
        }
        if binding.parent_canister_id == root.fleet_subnet_root
            || binding.parent_canister_id == root.authority.binding.coordinator
        {
            return Err(ComponentTopologyError::ChildParentConflictsWithAuthority);
        }

        let component_spec = self.get(&binding.component.component_spec).ok_or_else(|| {
            ComponentTopologyError::UnknownAdmissionSpec {
                component_spec: binding.component.component_spec.clone(),
            }
        })?;
        if !component_spec
            .children
            .iter()
            .any(|child| child.role == binding.role)
        {
            return Err(ComponentTopologyError::ChildRoleNotAdmitted {
                component_spec: component_spec.component_spec.clone(),
                role: binding.role.clone(),
            });
        }

        Ok(())
    }
}

fn validate_root_limits(limits: &FleetSubnetRootLimits) -> Result<(), ComponentTopologyError> {
    for (field, value) in [
        (
            "maximum_component_instances",
            u64::from(limits.maximum_component_instances),
        ),
        (
            "maximum_managed_canisters",
            u64::from(limits.maximum_managed_canisters),
        ),
        ("maximum_registry_bytes", limits.maximum_registry_bytes),
        ("maximum_wasm_store_bytes", limits.maximum_wasm_store_bytes),
        (
            "cycles_funding.window_secs",
            limits.cycles_funding.window_secs,
        ),
    ] {
        if value == 0 {
            return Err(ComponentTopologyError::NonPositiveRootLimit { field });
        }
    }
    if limits.cycles_funding.maximum_cycles.to_u128() == 0 {
        return Err(ComponentTopologyError::NonPositiveRootLimit {
            field: "cycles_funding.maximum_cycles",
        });
    }

    Ok(())
}

fn find_admission<'a>(
    admissions: &'a [ComponentSpecAdmission],
    component_spec: &crate::ids::ComponentSpecId,
) -> Option<&'a ComponentSpecAdmission> {
    admissions
        .binary_search_by(|admission| admission.component_spec.cmp(component_spec))
        .ok()
        .map(|index| &admissions[index])
}

fn require_nonanonymous(
    principal: &Principal,
    field: &'static str,
) -> Result<(), ComponentTopologyError> {
    if principal == &Principal::anonymous() {
        Err(ComponentTopologyError::AnonymousBindingPrincipal { field })
    } else {
        Ok(())
    }
}
