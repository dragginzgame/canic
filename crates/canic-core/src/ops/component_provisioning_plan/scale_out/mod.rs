//! Module: ops::component_provisioning_plan::scale_out
//!
//! Responsibility: validate one scale-out addition against durable placement authority.
//! Does not own: placement selection, persistence, root effects, publication, or receipts.
//! Boundary: committed placements and the initial installed-root set constrain one new plan.

use crate::{
    config::{
        ComponentDeploymentPurpose, ComponentGroupDeploymentSpec, ComponentGroupDeploymentTopology,
        FleetServiceMemberPurpose, FleetServiceTopology,
    },
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
        },
        fleet_registry::{FleetRegistry, FleetSubnetRootStatus},
    },
    ops::component_provisioning_plan::{
        ComponentProvisioningPlanOpsError, PlanValidationLedger, validate_spec_admissions,
    },
};
use std::collections::BTreeMap;

use candid::Principal;

/// One Coordinator-committed placement used to validate a scale-out addition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentProvisioningPlacementAuthority {
    pub placement: crate::ids::ComponentGroupPlacementId,
    pub fleet_subnet_root: Principal,
}

/// Durable Coordinator facts required to validate one monotonic scale-out plan.
#[derive(Clone, Copy, Debug)]
pub struct ComponentProvisioningScaleOutAuthority<'a> {
    pub committed_placements: &'a [ComponentProvisioningPlacementAuthority],
    pub eligible_roots: &'a [Principal],
    pub next_placement_ordinal: u32,
}

pub(super) fn seed_authority(
    ledger: &mut PlanValidationLedger,
    topology: &ComponentGroupDeploymentTopology,
    authority: ComponentProvisioningScaleOutAuthority<'_>,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    validate_canonical_eligible_roots(authority.eligible_roots)?;
    let mut previous = None;
    for placement in authority.committed_placements {
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &placement.placement)
        {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalCommittedPlacements);
        }
        previous = Some(placement.placement.clone());
        if authority
            .eligible_roots
            .binary_search(&placement.fleet_subnet_root)
            .is_err()
        {
            return Err(ComponentProvisioningPlanOpsError::ScaleOutRootIneligible);
        }
        let deployment = topology
            .get(&placement.placement.deployment)
            .ok_or_else(|| ComponentProvisioningPlanOpsError::UnknownDeployment {
                deployment: placement.placement.deployment.clone(),
            })?;
        if !ledger.placements.insert(placement.placement.clone()) {
            return Err(ComponentProvisioningPlanOpsError::DuplicatePlacement {
                placement: placement.placement.clone(),
            });
        }
        ledger.record(
            &placement.placement,
            deployment,
            placement.fleet_subnet_root,
        )?;
    }
    Ok(())
}

fn validate_canonical_eligible_roots(
    eligible_roots: &[Principal],
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut previous = None;
    for root in eligible_roots {
        if *root == Principal::anonymous() || previous.is_some_and(|previous| previous >= *root) {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalEligibleRoots);
        }
        previous = Some(*root);
    }
    Ok(())
}

pub(super) fn validate(
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    topology: &ComponentGroupDeploymentTopology,
    service_topology: &FleetServiceTopology,
    ledger: &PlanValidationLedger,
    authority: ComponentProvisioningScaleOutAuthority<'_>,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &plan.operation
    else {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutStateUnavailable);
    };
    let configured = topology.get(deployment).ok_or_else(|| {
        ComponentProvisioningPlanOpsError::UnknownDeployment {
            deployment: deployment.clone(),
        }
    })?;
    if configured.members.iter().any(|member| {
        matches!(
            member.purpose,
            ComponentDeploymentPurpose::FleetServiceMember {
                member_purpose: FleetServiceMemberPurpose::Authority,
                ..
            }
        )
    }) {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutAuthorityDeployment);
    }

    let committed_count = authority
        .committed_placements
        .iter()
        .filter(|placement| &placement.placement.deployment == deployment)
        .count();
    let committed_count = u32::try_from(committed_count)
        .map_err(|_| ComponentProvisioningPlanOpsError::CountOverflow)?;
    let requested_delta = requested_placements
        .checked_sub(*previous_placements)
        .filter(|delta| *delta > 0)
        .ok_or(ComponentProvisioningPlanOpsError::ScaleOutCountMismatch)?;
    if committed_count != *previous_placements
        || *requested_placements > configured.maximum_placements
    {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutCountMismatch);
    }

    validate_new_placement_ids(plan, deployment, requested_delta, authority)?;
    for batch in &plan.batches {
        if authority
            .eligible_roots
            .binary_search(&batch.root.fleet_subnet_root)
            .is_err()
        {
            return Err(ComponentProvisioningPlanOpsError::ScaleOutRootIneligible);
        }
    }
    validate_root_capacity(registry, ledger)?;
    validate_deployment_policy(configured, ledger, *requested_placements)?;
    validate_service_policy(service_topology, ledger)?;
    validate_confirmation_roots(plan, configured, ledger)
}

fn validate_new_placement_ids(
    plan: &FleetComponentProvisioningPlan,
    deployment: &crate::ids::ComponentGroupDeploymentId,
    requested_delta: u32,
    authority: ComponentProvisioningScaleOutAuthority<'_>,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut placements = plan
        .batches
        .iter()
        .flat_map(|batch| &batch.placements)
        .map(|placement| placement.group_placement.clone())
        .collect::<Vec<_>>();
    if placements.len() != requested_delta as usize {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutPlacementSetMismatch);
    }
    if placements
        .iter()
        .any(|placement| &placement.deployment != deployment)
    {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutDeploymentMismatch);
    }
    placements.sort_unstable();
    for (offset, placement) in placements.iter().enumerate() {
        let offset =
            u32::try_from(offset).map_err(|_| ComponentProvisioningPlanOpsError::CountOverflow)?;
        let expected = authority
            .next_placement_ordinal
            .checked_add(offset)
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        if placement.ordinal != expected {
            return Err(ComponentProvisioningPlanOpsError::ScaleOutPlacementSetMismatch);
        }
    }
    authority
        .next_placement_ordinal
        .checked_add(requested_delta)
        .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
    Ok(())
}

fn validate_root_capacity(
    registry: &FleetRegistry,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut placement_counts = BTreeMap::<Principal, u32>::new();
    for root in ledger.placement_roots.values() {
        let count = placement_counts.entry(*root).or_default();
        *count = count
            .checked_add(1)
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
    }
    for (root, placement_count) in placement_counts {
        let registry_root = registry
            .fleet_subnet_roots
            .iter()
            .find(|candidate| candidate.fleet_subnet_root == root)
            .filter(|candidate| candidate.status == FleetSubnetRootStatus::Active)
            .ok_or(ComponentProvisioningPlanOpsError::ScaleOutRootIneligible)?;
        if placement_count > registry_root.limits.maximum_group_placements {
            return Err(ComponentProvisioningPlanOpsError::RootGroupPlacementCapacityExceeded);
        }
        let component_count = ledger
            .root_component_counts
            .get(&root)
            .copied()
            .unwrap_or_default();
        if component_count > registry_root.limits.maximum_component_instances {
            return Err(ComponentProvisioningPlanOpsError::RootComponentCapacityExceeded);
        }
        let spec_counts = ledger
            .root_spec_counts
            .get(&root)
            .cloned()
            .unwrap_or_default();
        validate_spec_admissions(&registry_root.component_admissions, &spec_counts)?;
    }
    Ok(())
}

fn validate_deployment_policy(
    deployment: &ComponentGroupDeploymentSpec,
    ledger: &PlanValidationLedger,
    requested_placements: u32,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let roots = ledger.root_counts(&deployment.deployment);
    if roots
        .values()
        .any(|count| *count > deployment.placement.maximum_per_root)
    {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutPlacementPolicyMismatch);
    }
    let required_roots = requested_placements.min(deployment.placement.minimum_distinct_roots);
    if roots.len() < required_roots as usize {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutPlacementPolicyMismatch);
    }
    Ok(())
}

fn validate_service_policy(
    topology: &FleetServiceTopology,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    for target in &topology.targets {
        let roots = ledger.service_root_counts(&target.service);
        if roots
            .values()
            .any(|count| *count > target.placement.maximum_members_per_root)
        {
            return Err(ComponentProvisioningPlanOpsError::ScaleOutServicePlacementPolicyMismatch);
        }
        let members = roots.values().try_fold(0_u32, |total, count| {
            total
                .checked_add(*count)
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)
        })?;
        let required_roots = members.min(target.placement.minimum_distinct_roots);
        if roots.len() < required_roots as usize {
            return Err(ComponentProvisioningPlanOpsError::ScaleOutServicePlacementPolicyMismatch);
        }
    }
    Ok(())
}

fn validate_confirmation_roots(
    plan: &FleetComponentProvisioningPlan,
    deployment: &ComponentGroupDeploymentSpec,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut expected = plan
        .batches
        .iter()
        .map(|batch| batch.root.fleet_subnet_root)
        .collect::<std::collections::BTreeSet<_>>();
    for member in &deployment.members {
        let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &member.purpose else {
            continue;
        };
        expected.extend(ledger.service_root_counts(service).into_keys());
    }
    if plan
        .directory_confirmation_roots
        .iter()
        .copied()
        .ne(expected)
    {
        return Err(ComponentProvisioningPlanOpsError::ScaleOutConfirmationRootSetMismatch);
    }
    Ok(())
}
