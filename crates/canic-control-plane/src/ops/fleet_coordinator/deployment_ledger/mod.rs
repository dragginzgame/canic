//! Module: ops::fleet_coordinator::deployment_ledger
//!
//! Responsibility: materialize and validate Coordinator-owned group placement authority.
//! Does not own: placement selection, root effects, Component inventory, or scale-out policy.
//! Boundary: terminal root evidence becomes one canonical deployment ledger atomically.

#[cfg(test)]
mod tests;

use crate::{
    ops::fleet_coordinator::receipt_invariant,
    storage::stable::fleet_coordinator::{
        FleetComponentGroupDeploymentRecord, FleetComponentGroupPlacementRecord,
        FleetComponentProvisioningRecord, FleetComponentProvisioningStateRecord,
    },
};
use std::collections::BTreeMap;

use canic_core::{
    control_plane_support::{config::ComponentDeploymentConfiguration, error::InternalError},
    dto::component_provisioning::FleetComponentProvisioningOperation,
    ids::ComponentGroupDeploymentId,
};

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
        validate_initial_placement_set(
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

pub(super) fn validate(
    configuration: &ComponentDeploymentConfiguration,
    provisioning: Option<&FleetComponentProvisioningRecord>,
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<(), InternalError> {
    let Some(provisioning) = provisioning else {
        if deployments.is_empty() {
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
        let expected = compile_initial(configuration, provisioning)?;
        if deployments != expected {
            return Err(receipt_invariant(
                "deployment ledger differs from terminal fresh-install authority",
            ));
        }
        return Ok(());
    }
    if deployments.is_empty() {
        Ok(())
    } else {
        Err(receipt_invariant(
            "deployment ledger exists before terminal fresh installation",
        ))
    }
}

fn validate_initial_placement_set(
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
