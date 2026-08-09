//! Module: ops::fleet_coordinator::deployment_ledger
//!
//! Responsibility: materialize and reserve Coordinator-owned group placement authority.
//! Does not own: placement selection, root effects, Component inventory, or scale-out policy.
//! Boundary: terminal root evidence commits placements; a validated plan advances ordinals first.

#[cfg(test)]
mod tests;

use crate::{
    ops::fleet_coordinator::receipt_invariant,
    storage::stable::fleet_coordinator::{
        FleetComponentGroupDeploymentRecord, FleetComponentGroupPlacementRecord,
        FleetComponentProvisioningRecord, FleetComponentProvisioningStateRecord,
        FleetComponentScaleOutReceiptRecord,
    },
};
use std::collections::BTreeMap;

use canic_core::{
    control_plane_support::{
        config::ComponentDeploymentConfiguration,
        error::InternalError,
        ops::component_provisioning_plan::{
            ComponentProvisioningPlacementAuthority, ComponentProvisioningPlanOps,
            ComponentProvisioningScaleOutAuthority,
        },
    },
    dto::component_provisioning::{
        FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
    },
    dto::fleet_registry::FleetRegistry,
    ids::ComponentGroupDeploymentId,
};

pub(super) fn scale_out_plan_hash(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    fresh: &FleetComponentProvisioningRecord,
    deployments: &[FleetComponentGroupDeploymentRecord],
    plan: &FleetComponentProvisioningPlan,
) -> Result<[u8; 32], InternalError> {
    let deployment = scale_out_deployment(plan)?;
    let next_placement_ordinal = deployments
        .iter()
        .find(|candidate| &candidate.deployment == deployment)
        .map(|candidate| candidate.next_placement_ordinal)
        .ok_or_else(|| {
            InternalError::invalid_input("scale-out plan names an unknown deployment")
        })?;
    hash_with_next_ordinal(
        configuration,
        registry,
        fresh,
        deployments,
        plan,
        next_placement_ordinal,
    )
}

fn hash_with_next_ordinal(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    fresh: &FleetComponentProvisioningRecord,
    deployments: &[FleetComponentGroupDeploymentRecord],
    plan: &FleetComponentProvisioningPlan,
    next_placement_ordinal: u32,
) -> Result<[u8; 32], InternalError> {
    let committed_placements = committed_placement_authority(deployments);
    let eligible_roots = fresh_install_root_authority(fresh)?;
    ComponentProvisioningPlanOps::hash_scale_out_compiled(
        configuration,
        registry,
        plan,
        ComponentProvisioningScaleOutAuthority {
            committed_placements: &committed_placements,
            eligible_roots: &eligible_roots,
            next_placement_ordinal,
        },
    )
}

fn committed_placement_authority(
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Vec<ComponentProvisioningPlacementAuthority> {
    deployments
        .iter()
        .flat_map(|deployment| &deployment.placements)
        .map(|placement| ComponentProvisioningPlacementAuthority {
            placement: placement.placement.clone(),
            fleet_subnet_root: placement.fleet_subnet_root,
        })
        .collect()
}

fn fresh_install_root_authority(
    fresh: &FleetComponentProvisioningRecord,
) -> Result<Vec<candid::Principal>, InternalError> {
    if fresh.plan.operation != FleetComponentProvisioningOperation::FreshInstall {
        return Err(receipt_invariant(
            "installed-root authority does not come from fresh provisioning",
        ));
    }
    let mut roots = fresh
        .plan
        .batches
        .iter()
        .map(|batch| batch.root.fleet_subnet_root)
        .collect::<Vec<_>>();
    roots.sort_unstable();
    if roots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(receipt_invariant(
            "fresh provisioning contains duplicate installed-root authority",
        ));
    }
    Ok(roots)
}

fn scale_out_deployment(
    plan: &FleetComponentProvisioningPlan,
) -> Result<&ComponentGroupDeploymentId, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut { deployment, .. } = &plan.operation else {
        return Err(InternalError::invalid_input(
            "deployment reservation requires a scale-out operation",
        ));
    };
    Ok(deployment)
}

pub(super) fn reserve_scale_out(
    deployments: &[FleetComponentGroupDeploymentRecord],
    plan: &FleetComponentProvisioningPlan,
) -> Result<Vec<FleetComponentGroupDeploymentRecord>, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &plan.operation
    else {
        return Err(receipt_invariant(
            "deployment reservation requires a scale-out operation",
        ));
    };
    let mut next = deployments.to_vec();
    let target = next
        .iter_mut()
        .find(|candidate| &candidate.deployment == deployment)
        .ok_or_else(|| receipt_invariant("scale-out deployment ledger is absent"))?;
    let current_count = u32::try_from(target.placements.len())
        .map_err(|_| receipt_invariant("committed placement count does not fit u32"))?;
    let reservation_count = requested_placements
        .checked_sub(*previous_placements)
        .filter(|count| *count > 0)
        .ok_or_else(|| receipt_invariant("scale-out placement count is not monotonic"))?;
    if current_count != *previous_placements || *requested_placements > target.maximum_placements {
        return Err(receipt_invariant(
            "scale-out desired count differs from the committed deployment ledger",
        ));
    }
    let reserved_end = target
        .next_placement_ordinal
        .checked_add(reservation_count)
        .ok_or_else(|| receipt_invariant("scale-out placement ordinal space is exhausted"))?;
    let plan_end = scale_out_plan_reserved_end(plan)?;
    if plan_end != reserved_end {
        return Err(receipt_invariant(
            "scale-out plan does not reserve the exact next ordinal range",
        ));
    }
    target.next_placement_ordinal = reserved_end;
    Ok(next)
}

pub(super) fn compile_initial(
    configuration: &ComponentDeploymentConfiguration,
    provisioning: &FleetComponentProvisioningRecord,
) -> Result<Vec<FleetComponentGroupDeploymentRecord>, InternalError> {
    if provisioning.plan.operation != FleetComponentProvisioningOperation::FreshInstall {
        return Err(receipt_invariant(
            "initial deployment ledger requires the fresh-install operation",
        ));
    }
    let FleetComponentProvisioningStateRecord::RuntimesActivated { activations, .. } =
        &provisioning.state
    else {
        return Err(receipt_invariant(
            "initial deployment ledger requires terminal runtime evidence",
        ));
    };
    if activations.len() != provisioning.plan.batches.len() {
        return Err(receipt_invariant(
            "initial deployment ledger lacks one terminal receipt per root batch",
        ));
    }

    let mut placements_by_deployment: BTreeMap<
        ComponentGroupDeploymentId,
        Vec<FleetComponentGroupPlacementRecord>,
    > = BTreeMap::new();
    for (batch, activation) in provisioning.plan.batches.iter().zip(activations) {
        if activation.progress.fleet_subnet_root != batch.root.fleet_subnet_root {
            return Err(receipt_invariant(
                "initial deployment ledger root receipt differs from its planned batch",
            ));
        }
        if activation.receipt_content_hash == [0; 32] {
            return Err(receipt_invariant(
                "initial deployment ledger root receipt hash is zero",
            ));
        }
        for planned in &batch.placements {
            let placement = FleetComponentGroupPlacementRecord {
                placement: planned.group_placement.clone(),
                fleet_subnet_root: batch.root.fleet_subnet_root,
                operation_id: provisioning.operation_id,
                plan_hash: provisioning.plan_hash,
                root_receipt_content_hash: activation.receipt_content_hash,
            };
            placements_by_deployment
                .entry(planned.group_placement.deployment.clone())
                .or_default()
                .push(placement);
        }
    }

    let mut deployments = Vec::with_capacity(
        configuration
            .deployment_topology
            .component_group_deployments
            .len(),
    );
    for configured in &configuration
        .deployment_topology
        .component_group_deployments
    {
        let mut placements = placements_by_deployment
            .remove(&configured.deployment)
            .unwrap_or_default();
        placements.sort_unstable_by(|left, right| left.placement.cmp(&right.placement));
        validate_contiguous_placement_set(
            &configured.deployment,
            configured.initial_placements,
            &placements,
        )?;
        deployments.push(FleetComponentGroupDeploymentRecord {
            deployment: configured.deployment.clone(),
            component_group: configured.component_group.clone(),
            configuration_digest: provisioning.plan.configuration_digest,
            initial_placements: configured.initial_placements,
            maximum_placements: configured.maximum_placements,
            placement_policy: configured.placement,
            next_placement_ordinal: configured.initial_placements,
            placements,
        });
    }
    if !placements_by_deployment.is_empty() {
        return Err(receipt_invariant(
            "initial deployment ledger contains an unknown deployment",
        ));
    }
    Ok(deployments)
}

pub(super) fn commit_scale_out(
    deployments: &[FleetComponentGroupDeploymentRecord],
    scale_out: &FleetComponentProvisioningRecord,
) -> Result<Vec<FleetComponentGroupDeploymentRecord>, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &scale_out.plan.operation
    else {
        return Err(receipt_invariant(
            "deployment commit requires a scale-out operation",
        ));
    };
    let FleetComponentProvisioningStateRecord::RuntimesActivated { activations, .. } =
        &scale_out.state
    else {
        return Err(receipt_invariant(
            "deployment commit requires terminal runtime evidence",
        ));
    };
    if activations.len() != scale_out.plan.batches.len() {
        return Err(receipt_invariant(
            "scale-out deployment commit lacks one terminal receipt per selected root",
        ));
    }

    let mut next = deployments.to_vec();
    let target = next
        .iter_mut()
        .find(|candidate| &candidate.deployment == deployment)
        .ok_or_else(|| receipt_invariant("scale-out deployment ledger is absent"))?;
    let committed_count = u32::try_from(target.placements.len())
        .map_err(|_| receipt_invariant("committed placement count does not fit u32"))?;
    let reservation_is_exact = committed_count == *previous_placements
        && target.next_placement_ordinal == *requested_placements
        && *requested_placements <= target.maximum_placements;
    if !reservation_is_exact {
        return Err(receipt_invariant(
            "scale-out deployment commit differs from its durable reservation",
        ));
    }

    for (batch, activation) in scale_out.plan.batches.iter().zip(activations) {
        let root_is_exact = activation.progress.fleet_subnet_root == batch.root.fleet_subnet_root;
        let progress_is_terminal = activation.progress.root_runtime_active
            && activation.progress.activated_component_count == activation.progress.component_count;
        let receipt_is_terminal = activation.activation.is_some()
            && activation.runtimes_activated_at_ns.is_some()
            && activation.receipt_content_hash != [0; 32];
        if !root_is_exact || !progress_is_terminal || !receipt_is_terminal {
            return Err(receipt_invariant(
                "scale-out deployment commit has invalid selected-root evidence",
            ));
        }
        for planned in &batch.placements {
            if &planned.group_placement.deployment != deployment {
                return Err(receipt_invariant(
                    "scale-out deployment commit contains an unrelated placement",
                ));
            }
            target.placements.push(FleetComponentGroupPlacementRecord {
                placement: planned.group_placement.clone(),
                fleet_subnet_root: batch.root.fleet_subnet_root,
                operation_id: scale_out.operation_id,
                plan_hash: scale_out.plan_hash,
                root_receipt_content_hash: activation.receipt_content_hash,
            });
        }
    }
    target
        .placements
        .sort_unstable_by(|left, right| left.placement.cmp(&right.placement));
    validate_contiguous_placement_set(deployment, *requested_placements, &target.placements)?;
    Ok(next)
}

pub(super) fn validate(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    provisioning: Option<&FleetComponentProvisioningRecord>,
    scale_out_receipts: &[FleetComponentScaleOutReceiptRecord],
    scale_out: Option<&FleetComponentProvisioningRecord>,
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<(), InternalError> {
    let Some(provisioning) = provisioning else {
        if deployments.is_empty() && scale_out_receipts.is_empty() && scale_out.is_none() {
            return Ok(());
        }
        return Err(receipt_invariant(
            "deployment ledger exists without a provisioning operation",
        ));
    };
    if matches!(
        provisioning.state,
        FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
    ) {
        let mut expected = compile_initial(configuration, provisioning)?;
        for receipt in scale_out_receipts {
            commit_scale_out_receipt(&mut expected, receipt)?;
        }
        validate_active_scale_out_record(
            configuration,
            registry,
            provisioning,
            scale_out,
            &expected,
        )?;
        if let Some(scale_out) = scale_out {
            expected = reserve_scale_out(&expected, &scale_out.plan)?;
            if matches!(
                scale_out.state,
                FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
            ) {
                expected = commit_scale_out(&expected, scale_out)?;
            }
        }
        if deployments == expected {
            return Ok(());
        }
        return Err(receipt_invariant(
            "deployment ledger differs from exact scale-out history and active authority",
        ));
    }
    if deployments.is_empty() && scale_out_receipts.is_empty() && scale_out.is_none() {
        Ok(())
    } else {
        Err(receipt_invariant(
            "deployment ledger exists before terminal fresh installation",
        ))
    }
}

fn validate_active_scale_out_record(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    fresh: &FleetComponentProvisioningRecord,
    scale_out: Option<&FleetComponentProvisioningRecord>,
    committed_deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<(), InternalError> {
    let Some(scale_out) = scale_out else {
        return Ok(());
    };
    if scale_out.operation_id == [0; 32] || scale_out.plan_hash == [0; 32] {
        return Err(receipt_invariant(
            "Fleet Component scale-out operation or plan hash is zero",
        ));
    }
    if scale_out.operation_id == fresh.operation_id {
        return Err(receipt_invariant(
            "Fleet Component scale-out reuses the fresh operation identity",
        ));
    }
    if !scale_out_runtime_boundary_is_valid(&scale_out.state) {
        return Err(receipt_invariant(
            "Fleet Component scale-out has an invalid runtime-activation boundary",
        ));
    }
    let plan_hash = hash_with_next_ordinal(
        configuration,
        registry,
        fresh,
        committed_deployments,
        &scale_out.plan,
        scale_out_plan_reserved_start(&scale_out.plan)?,
    )
    .map_err(|_| {
        receipt_invariant(
            "Fleet Component scale-out plan differs from placement, configuration or Registry authority",
        )
    })?;
    if scale_out.plan_hash != plan_hash {
        return Err(receipt_invariant(
            "Fleet Component scale-out plan hash differs from canonical bytes",
        ));
    }
    Ok(())
}

fn commit_scale_out_receipt(
    deployments: &mut [FleetComponentGroupDeploymentRecord],
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<(), InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &receipt.operation
    else {
        return Err(receipt_invariant(
            "deployment receipt contains a different operation kind",
        ));
    };
    let target_index = deployments
        .binary_search_by(|candidate| candidate.deployment.cmp(deployment))
        .map_err(|_| receipt_invariant("retired scale-out deployment ledger is absent"))?;
    let target = &mut deployments[target_index];
    let committed_count = u32::try_from(target.placements.len())
        .map_err(|_| receipt_invariant("committed placement count does not fit u32"))?;
    if committed_count != *previous_placements
        || target.next_placement_ordinal != *previous_placements
        || *requested_placements > target.maximum_placements
    {
        return Err(receipt_invariant(
            "retired scale-out receipt differs from prior deployment authority",
        ));
    }
    let expected_count = requested_placements
        .checked_sub(*previous_placements)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| receipt_invariant("retired scale-out count does not fit usize"))?;
    if receipt.placements.len() != expected_count {
        return Err(receipt_invariant(
            "retired scale-out receipt lacks its exact placement range",
        ));
    }
    for (offset, placement) in receipt.placements.iter().enumerate() {
        let offset = u32::try_from(offset)
            .map_err(|_| receipt_invariant("retired placement offset does not fit u32"))?;
        let expected_ordinal = previous_placements
            .checked_add(offset)
            .ok_or_else(|| receipt_invariant("retired placement ordinal overflowed"))?;
        if &placement.placement.deployment != deployment
            || placement.placement.ordinal != expected_ordinal
        {
            return Err(receipt_invariant(
                "retired scale-out receipt placement range is noncanonical",
            ));
        }
    }
    target.next_placement_ordinal = *requested_placements;
    target.placements.extend(receipt.placements.iter().cloned());
    Ok(())
}

const fn scale_out_runtime_boundary_is_valid(
    state: &FleetComponentProvisioningStateRecord,
) -> bool {
    match state {
        FleetComponentProvisioningStateRecord::Planned { planned_at_ns }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::RootsAccepted { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            planned_at_ns, ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes { planned_at_ns, .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { planned_at_ns, .. } => {
            *planned_at_ns > 0
        }
    }
}

#[cfg(test)]
fn validate_terminal_ledger(
    expected: &[FleetComponentGroupDeploymentRecord],
    scale_out: Option<&FleetComponentProvisioningRecord>,
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<(), InternalError> {
    let Some(scale_out) = scale_out else {
        return (deployments == expected)
            .then_some(())
            .ok_or_else(|| receipt_invariant("deployment ledger differs from expected authority"));
    };
    let reserved = reserve_scale_out(expected, &scale_out.plan)?;
    let authoritative = if matches!(
        scale_out.state,
        FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
    ) {
        commit_scale_out(&reserved, scale_out)?
    } else {
        reserved
    };
    (deployments == authoritative).then_some(()).ok_or_else(|| {
        receipt_invariant("deployment ledger differs from exact scale-out authority")
    })
}

fn scale_out_plan_reserved_end(
    plan: &FleetComponentProvisioningPlan,
) -> Result<u32, InternalError> {
    plan.batches
        .iter()
        .flat_map(|batch| &batch.placements)
        .map(|placement| placement.group_placement.ordinal)
        .max()
        .and_then(|last| last.checked_add(1))
        .ok_or_else(|| receipt_invariant("scale-out journal has no bounded reservation"))
}

fn scale_out_plan_reserved_start(
    plan: &FleetComponentProvisioningPlan,
) -> Result<u32, InternalError> {
    plan.batches
        .iter()
        .flat_map(|batch| &batch.placements)
        .map(|placement| placement.group_placement.ordinal)
        .min()
        .ok_or_else(|| receipt_invariant("scale-out journal has no bounded reservation"))
}

fn validate_contiguous_placement_set(
    deployment: &ComponentGroupDeploymentId,
    initial_placements: u32,
    placements: &[FleetComponentGroupPlacementRecord],
) -> Result<(), InternalError> {
    let expected_count = usize::try_from(initial_placements)
        .map_err(|_| receipt_invariant("initial placement count does not fit usize"))?;
    if placements.len() != expected_count {
        return Err(receipt_invariant(
            "initial deployment ledger placement count differs from configuration",
        ));
    }
    for (ordinal, placement) in (0..initial_placements).zip(placements) {
        if &placement.placement.deployment != deployment || placement.placement.ordinal != ordinal {
            return Err(receipt_invariant(
                "initial deployment ledger placements are not the canonical contiguous set",
            ));
        }
    }
    Ok(())
}
