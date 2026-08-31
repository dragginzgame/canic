//! Module: ops::fleet_coordinator::admission
//!
//! Responsibility: publish and replay the Coordinator-owned Fleet admission policy.
//! Does not own: admission endpoint authorization, participant convergence, or Registry storage.
//! Boundary: the Coordinator record remains the sole durable authority and this module compiles
//! one exact policy mutation into its canonical Registry history.

use super::{FleetCoordinatorOps, component_provisioning_status_response, receipt_invariant};
use crate::storage::stable::fleet_coordinator::{
    FleetAdmissionPublicationActionRecord, FleetAdmissionPublicationRecord,
    FleetCoordinatorFundingStore, FleetCoordinatorRegistryRecord,
};
use canic_core::{
    control_plane_support::{error::InternalError, ops::fleet_registry::FleetRegistryOps},
    dto::{
        component_provisioning::FleetComponentProvisioningPhase,
        fleet_registry::{FleetRegistry, FleetRegistryVersion, FleetSubnetRootStatus},
    },
    ids::FleetAdmissionPolicy,
    shared_support::{
        fleet_admission_authority::{
            FleetAdmissionMutationActionModel, FleetAdmissionMutationRequestModel,
            MAX_FLEET_ADMISSION_PUBLICATIONS, mutate_fleet_admission_membership,
        },
        fleet_admission_policy::compile_installed_fleet_admission_policy,
    },
};

impl FleetCoordinatorOps {
    /// Reject a new admission transition while another participant/Registry owner is active.
    pub(crate) fn require_admission_transition_start_allowed() -> Result<(), InternalError> {
        let current = Self::current()?;
        let component_operation_active = [
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(component_provisioning_status_response)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|status| status.phase != FleetComponentProvisioningPhase::RuntimesActivated);
        let root_removal_active = current
            .root_draining_reservations
            .iter()
            .any(|reservation| {
                current.registry.fleet_subnet_roots.iter().any(|root| {
                    root.fleet_subnet_root
                        == reservation.response.request.expected_root.fleet_subnet_root
                        && root.status == FleetSubnetRootStatus::Active
                })
            });
        let funding_rotation_active = FleetCoordinatorFundingStore::export()
            .current
            .is_some_and(|funding| funding.rotation_current.is_some());
        let admission_history_full =
            admission_publication_history_full(current.admission_publications.len());
        if component_operation_active
            || root_removal_active
            || funding_rotation_active
            || admission_history_full
        {
            Err(InternalError::conflict())
        } else {
            Ok(())
        }
    }

    /// Publish one exact successor admission generation in the canonical Registry.
    pub(crate) fn publish_admission_policy(
        request: FleetAdmissionMutationRequestModel,
        successor: FleetAdmissionPolicy,
    ) -> Result<FleetRegistry, InternalError> {
        let current = Self::current()?;
        let retained = current
            .admission_publications
            .iter()
            .find(|retained| retained.operation_id == request.operation_id);
        let receipt = match retained {
            Some(retained)
                if admission_publication_matches_request(retained, &request, &successor) =>
            {
                retained.clone()
            }
            Some(_) => return Err(InternalError::conflict()),
            None => {
                if admission_publication_history_full(current.admission_publications.len()) {
                    return Err(InternalError::resource_exhausted());
                }
                let receipt = admission_publication_receipt(&current, &request, &successor)?;
                let mut next = current.clone();
                next.admission_publications.push(receipt.clone());
                let next = Self::validate_current(next)?;
                Self::commit_transition(&current, next)?;
                receipt
            }
        };

        let current = Self::current()?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if current_version == receipt.version && current.registry.admission == successor {
            return Ok(current.registry);
        }
        if current_version != receipt.previous_version {
            return Err(InternalError::conflict());
        }
        let mut next = current.clone();
        next.registry = registry_after_admission_publication(&current, &receipt)?;
        let next = Self::validate_current(next)?;
        let registry = next.registry.clone();
        Self::commit_transition(&current, next)?;
        Ok(registry)
    }
}

fn admission_publication_receipt(
    current: &FleetCoordinatorRegistryRecord,
    request: &FleetAdmissionMutationRequestModel,
    successor: &FleetAdmissionPolicy,
) -> Result<FleetAdmissionPublicationRecord, InternalError> {
    if request.operation_id == [0; 32]
        || request.authority != current.authority.binding
        || request.expected_generation != current.registry.admission.generation
        || request.expected_policy_digest != current.registry.admission.policy_digest
        || request.successor_policy_digest != successor.policy_digest
        || successor.fleet != current.authority.binding.fleet
    {
        return Err(InternalError::conflict());
    }
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
    )?;
    let mut receipt = FleetAdmissionPublicationRecord {
        operation_id: request.operation_id,
        action: admission_publication_action(request.action),
        selector: request.selector.clone(),
        principal: request.principal,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        successor_generation: successor.generation,
        successor_policy_digest: successor.policy_digest,
        previous_version,
        version: FleetRegistryVersion {
            authority: current.authority.clone(),
            revision: 0,
            content_hash: [0; 32],
        },
    };
    let next = apply_admission_publication_to_registry(current, &current.registry, &receipt)?;
    receipt.version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next,
    )?;
    Ok(receipt)
}

fn admission_publication_matches_request(
    receipt: &FleetAdmissionPublicationRecord,
    request: &FleetAdmissionMutationRequestModel,
    successor: &FleetAdmissionPolicy,
) -> bool {
    receipt.operation_id == request.operation_id
        && receipt.action == admission_publication_action(request.action)
        && receipt.selector == request.selector
        && receipt.principal == request.principal
        && receipt.expected_generation == request.expected_generation
        && receipt.expected_policy_digest == request.expected_policy_digest
        && receipt.successor_generation == successor.generation
        && receipt.successor_policy_digest == successor.policy_digest
        && successor.fleet == request.authority.fleet
}

const fn admission_publication_action(
    action: FleetAdmissionMutationActionModel,
) -> FleetAdmissionPublicationActionRecord {
    match action {
        FleetAdmissionMutationActionModel::Add => FleetAdmissionPublicationActionRecord::Add,
        FleetAdmissionMutationActionModel::Remove => FleetAdmissionPublicationActionRecord::Remove,
    }
}

const fn admission_publication_action_model(
    action: FleetAdmissionPublicationActionRecord,
) -> FleetAdmissionMutationActionModel {
    match action {
        FleetAdmissionPublicationActionRecord::Add => FleetAdmissionMutationActionModel::Add,
        FleetAdmissionPublicationActionRecord::Remove => FleetAdmissionMutationActionModel::Remove,
    }
}

pub(super) fn apply_admission_publication_to_registry(
    current: &FleetCoordinatorRegistryRecord,
    source: &FleetRegistry,
    receipt: &FleetAdmissionPublicationRecord,
) -> Result<FleetRegistry, InternalError> {
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        source,
    )?;
    if receipt.operation_id == [0; 32]
        || receipt.previous_version != previous_version
        || receipt.expected_generation != source.admission.generation
        || receipt.expected_policy_digest != source.admission.policy_digest
        || receipt.successor_generation
            != receipt
                .expected_generation
                .checked_add(1)
                .ok_or_else(InternalError::invariant)?
    {
        return Err(receipt_invariant(
            "Fleet admission publication source differs from canonical history",
        ));
    }
    let membership = mutate_fleet_admission_membership(
        &source.admission,
        admission_publication_action_model(receipt.action),
        &receipt.selector,
        receipt.principal,
    )
    .map_err(|_error| receipt_invariant("Fleet admission publication mutation is invalid"))?;
    if !membership.changed {
        return Err(receipt_invariant(
            "Fleet admission publication retained a no-op mutation",
        ));
    }
    let successor = compile_installed_fleet_admission_policy(
        source.admission.fleet.clone(),
        receipt.successor_generation,
        membership.fleet_principals,
        membership.rules,
    )
    .map_err(|_error| receipt_invariant("Fleet admission successor cannot be recompiled"))?;
    if successor.policy_digest != receipt.successor_policy_digest {
        return Err(receipt_invariant(
            "Fleet admission successor digest differs from canonical mutation",
        ));
    }
    let mut next = source.clone();
    next.admission = successor;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    if receipt.version.revision != 0 {
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next,
        )?;
        if version != receipt.version {
            return Err(receipt_invariant(
                "Fleet admission publication target differs from canonical mutation",
            ));
        }
    }
    Ok(next)
}

fn registry_after_admission_publication(
    current: &FleetCoordinatorRegistryRecord,
    receipt: &FleetAdmissionPublicationRecord,
) -> Result<FleetRegistry, InternalError> {
    apply_admission_publication_to_registry(current, &current.registry, receipt)
}

const fn admission_publication_history_full(publication_count: usize) -> bool {
    publication_count >= MAX_FLEET_ADMISSION_PUBLICATIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_publication_limit_rejects_identity_4097_before_publication() {
        assert!(!admission_publication_history_full(
            MAX_FLEET_ADMISSION_PUBLICATIONS - 1
        ));
        assert!(admission_publication_history_full(
            MAX_FLEET_ADMISSION_PUBLICATIONS
        ));
        assert!(admission_publication_history_full(
            MAX_FLEET_ADMISSION_PUBLICATIONS + 1
        ));
    }
}
