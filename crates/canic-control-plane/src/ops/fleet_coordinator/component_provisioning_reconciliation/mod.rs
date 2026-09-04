//! Root acceptance and provisioning response reconciliation for the Fleet Coordinator.
//!
//! Boundary: the Coordinator retains the journal; this owner validates one observed Root response.

use super::*;

#[derive(Eq, PartialEq)]
struct RootAcceptanceResponseIdentity<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fleet_subnet_root: Principal,
}

#[derive(Eq, PartialEq)]
struct RootAcceptanceResponseProgress<'a> {
    phase: RootComponentProvisioningPhase,
    placement_count: u32,
    component_count: u32,
    reserved_component_count: u32,
    claimed_component_count: u32,
    installed_component_count: u32,
    registry_committed_component_count: u32,
    result: Option<&'a canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    provisioned_at_ns: Option<u64>,
}

pub(super) struct RootProvisionResponseValidation<'a> {
    pub(super) configuration:
        &'a canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    pub(super) record: &'a FleetComponentProvisioningRecord,
    pub(super) root_index: u32,
    pub(super) acceptance: &'a FleetComponentProvisioningRootAcceptanceRecord,
    pub(super) previous: &'a RootComponentProvisioningStatusResponse,
    pub(super) response: &'a RootComponentProvisioningStatusResponse,
    pub(super) recorded_at_ns: u64,
}

pub(super) fn canonical_root_acceptance_observation(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    batch: &FleetSubnetRootProvisioningBatch,
    observed: &RootComponentProvisioningStatusResponse,
    started_at_ns: u64,
    recorded_at_ns: u64,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let canonical = canonical_root_acceptance_response(record, batch, observed.accepted_at_ns)?;
    let acceptance = FleetComponentProvisioningRootAcceptanceRecord {
        started_at_ns,
        response: canonical.clone(),
        recorded_at_ns,
    };
    match observed.phase {
        RootComponentProvisioningPhase::Accepted => {
            validate_root_provision_current(record, batch, &acceptance, observed)?;
        }
        RootComponentProvisioningPhase::Provisioned => {
            FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
                configuration,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                usize::try_from(root_index).map_err(|_| InternalError::resource_exhausted())?,
                observed,
            )?;
            if !root_post_provisioning_progress_is_absent(observed) {
                return Err(InternalError::conflict());
            }
            let provisioned_at_ns = observed
                .provisioned_at_ns
                .ok_or_else(InternalError::conflict)?;
            if recorded_at_ns < provisioned_at_ns {
                return Err(InternalError::invalid_input());
            }
        }
        RootComponentProvisioningPhase::Published
        | RootComponentProvisioningPhase::RuntimesActive => {
            return Err(InternalError::conflict());
        }
    }
    validate_root_acceptance_response(record, batch, &canonical)?;
    validate_root_acceptance_observation(started_at_ns, &canonical, recorded_at_ns)?;
    Ok(canonical)
}

fn canonical_root_acceptance_response(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    accepted_at_ns: u64,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (placement_count, component_count) = root_batch_counts(batch)?;
    let receipt_content_hash = RootComponentProvisioningReceiptOps::acceptance_content_hash(
        RootComponentProvisioningAcceptanceReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            fleet_registry: &record.plan.fleet_registry,
            configuration_digest: record.plan.configuration_digest,
            batch,
            placement_count,
            component_count,
            accepted_at_ns,
        },
    )?;
    Ok(RootComponentProvisioningStatusResponse {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
        estate_funding_required: None,
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count,
        component_count,
        reserved_component_count: 0,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
        published_component_count: 0,
        activated_component_count: 0,
        root_runtime_active: false,
        result: None,
        publication: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: None,
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    })
}

pub(super) fn validate_root_acceptance_response(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let expected_identity = RootAcceptanceResponseIdentity {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: &record.plan.fleet_registry,
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_identity = RootAcceptanceResponseIdentity {
        operation_id: response.operation_id,
        plan_hash: response.plan_hash,
        fleet_registry: &response.fleet_registry,
        configuration_digest: response.configuration_digest,
        fleet_subnet_root: response.fleet_subnet_root,
    };
    if actual_identity != expected_identity {
        return Err(InternalError::conflict());
    }
    let (placement_count, component_count) = root_batch_counts(batch)?;
    let expected_progress = RootAcceptanceResponseProgress {
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count,
        component_count,
        reserved_component_count: 0,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
        result: None,
        provisioned_at_ns: None,
    };
    let actual_progress = RootAcceptanceResponseProgress {
        phase: response.phase,
        placement_count: response.placement_count,
        component_count: response.component_count,
        reserved_component_count: response.reserved_component_count,
        claimed_component_count: response.claimed_component_count,
        installed_component_count: response.installed_component_count,
        registry_committed_component_count: response.registry_committed_component_count,
        result: response.result.as_ref(),
        provisioned_at_ns: response.provisioned_at_ns,
    };
    if actual_progress != expected_progress {
        return Err(InternalError::conflict());
    }
    if response.accepted_at_ns == 0 {
        return Err(InternalError::conflict());
    }
    let receipt_content_hash = RootComponentProvisioningReceiptOps::acceptance_content_hash(
        RootComponentProvisioningAcceptanceReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            fleet_registry: &record.plan.fleet_registry,
            configuration_digest: record.plan.configuration_digest,
            batch,
            placement_count,
            component_count,
            accepted_at_ns: response.accepted_at_ns,
        },
    )?;
    if response.receipt_content_hash != receipt_content_hash {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) const fn validate_root_acceptance_observation(
    started_at_ns: u64,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if response.accepted_at_ns < started_at_ns || recorded_at_ns < response.accepted_at_ns {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

pub(super) fn validate_root_provision_response(
    validation: RootProvisionResponseValidation<'_>,
) -> Result<(), InternalError> {
    let RootProvisionResponseValidation {
        configuration,
        record,
        root_index,
        acceptance,
        previous,
        response,
        recorded_at_ns,
    } = validation;
    let batch = root_batch(record, root_index)?;
    if previous.phase != RootComponentProvisioningPhase::Accepted {
        return Err(receipt_invariant(
            "root provisioning predecessor is not in the Accepted phase",
        ));
    }
    if response.accepted_at_ns != acceptance.response.accepted_at_ns {
        return Err(InternalError::conflict());
    }
    match response.phase {
        RootComponentProvisioningPhase::Accepted => {
            validate_root_provision_current(record, batch, acceptance, response)?;
            let previous_counts = RootProvisioningCounts::from_response(previous);
            let next_counts = RootProvisioningCounts::from_response(response);
            if !previous_counts.advances_one_step_to(next_counts, response.component_count) {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::Provisioned => {
            FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
                configuration,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                usize::try_from(root_index).map_err(|_| InternalError::resource_exhausted())?,
                response,
            )?;
            if !root_post_provisioning_progress_is_absent(response) {
                return Err(InternalError::conflict());
            }
            let provisioned_at_ns = response
                .provisioned_at_ns
                .ok_or_else(InternalError::conflict)?;
            if recorded_at_ns < provisioned_at_ns {
                return Err(InternalError::invalid_input());
            }
        }
        RootComponentProvisioningPhase::Published
        | RootComponentProvisioningPhase::RuntimesActive => {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

pub(super) fn validate_root_provision_current(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    acceptance: &FleetComponentProvisioningRootAcceptanceRecord,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let expected_identity = RootAcceptanceResponseIdentity {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: &record.plan.fleet_registry,
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_identity = RootAcceptanceResponseIdentity {
        operation_id: response.operation_id,
        plan_hash: response.plan_hash,
        fleet_registry: &response.fleet_registry,
        configuration_digest: response.configuration_digest,
        fleet_subnet_root: response.fleet_subnet_root,
    };
    let (placement_count, component_count) = root_batch_counts(batch)?;
    let progress_is_valid = response.phase == RootComponentProvisioningPhase::Accepted
        && response.placement_count == placement_count
        && response.component_count == component_count
        && response.result.is_none()
        && response.provisioned_at_ns.is_none()
        && root_post_provisioning_progress_is_absent(response)
        && RootProvisioningCounts::from_response(response).is_canonical(component_count);
    let acceptance_is_exact = response.accepted_at_ns == acceptance.response.accepted_at_ns
        && response.receipt_content_hash == acceptance.response.receipt_content_hash;
    if actual_identity != expected_identity || !progress_is_valid || !acceptance_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) fn root_post_provisioning_progress_is_absent(
    response: &RootComponentProvisioningStatusResponse,
) -> bool {
    [
        response.published_component_count == 0,
        response.activated_component_count == 0,
        !response.root_runtime_active,
        response.publication.is_none(),
        response.activation.is_none(),
        response.published_at_ns.is_none(),
        response.activation_started_at_ns.is_none(),
        response.runtimes_activated_at_ns.is_none(),
    ]
    .into_iter()
    .all(|is_absent| is_absent)
}

fn root_batch_counts(
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<(u32, u32), InternalError> {
    let placement_count = u32::try_from(batch.placements.len())
        .map_err(|_| receipt_invariant("root batch placement count does not fit u32"))?;
    let mut component_count = 0_u32;
    for placement in &batch.placements {
        let members = u32::try_from(placement.entries.len())
            .map_err(|_| receipt_invariant("root batch member count does not fit u32"))?;
        component_count = component_count
            .checked_add(members)
            .ok_or_else(|| receipt_invariant("root batch Component count overflowed"))?;
    }
    Ok((placement_count, component_count))
}
