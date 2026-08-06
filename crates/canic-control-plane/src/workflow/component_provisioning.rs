//! Module: workflow::component_provisioning
//!
//! Responsibility: authenticate and advance one exact Coordinator-planned root batch.
//! Does not own: stable records, pool state, service publication, or runtime activation.
//! Boundary: acceptance and each bounded member step revalidate protected root, Registry, config,
//! Store and aggregate progress before delegating to existing root-local lifecycle authority.

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, component_provisioning::RootComponentProvisioningOps,
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::{
        component_provisioning::{
            RootComponentProvisioningAdvanceDisposition, RootComponentProvisioningMemberView,
            RootComponentProvisioningView,
        },
        component_registry::{RootComponentAllocationView, RootComponentRegistryView},
    },
    workflow::{bootstrap::root_store, root_authority::validated_root_authority},
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{
            component_provisioning_plan::{
                ComponentProvisioningPlanOps, RootComponentProvisioningBatchValidation,
            },
            config::ConfigOps,
            ic::IcOps,
        },
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        component_provisioning::{
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningStatusRequest, RootComponentProvisioningStatusResponse,
        },
        component_registry::{ComponentProvisioningOrigin, RootComponentAllocationRequest},
        fleet_activation::FleetActivationPhase,
        fleet_registry::FleetSubnetRootStatus,
        fleet_subnet_root::FleetSubnetRootAuthority,
    },
};

/// Durably accept one complete root batch under the exact protected Coordinator.
pub async fn accept(
    caller: Principal,
    request: RootComponentProvisioningAcceptanceRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    if let Some(existing) = RootComponentProvisioningOps::acceptance_replay(&request)? {
        return Ok(crate::ops::component_provisioning::status_response(
            existing,
        ));
    }
    RootComponentProvisioningOps::require_acceptance_open(request.operation_id)?;
    if FleetActivationWorkflow::status()?.phase != FleetActivationPhase::Prepared {
        return Err(InternalError::conflict(
            "fresh root Component provisioning acceptance requires runtime Prepared",
        ));
    }

    let mirror = FleetRegistryMirrorOps::validated_current(&authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != request.fleet_registry
    {
        return Err(InternalError::conflict(
            "root Component provisioning request differs from the exact active Registry mirror",
        ));
    }
    let config = ConfigOps::get()?;
    let validation = ComponentProvisioningPlanOps::validate_root_batch(
        &config,
        &mirror.active.snapshot.registry,
        &request.fleet_registry,
        request.configuration_digest,
        &authority.binding,
        &request.batch,
    )?;
    let _canonical_batch = ComponentProvisioningPlanOps::root_batch_canonical_bytes(
        &config,
        &mirror.active.snapshot.registry,
        &request.fleet_registry,
        request.configuration_digest,
        &authority.binding,
        &request.batch,
    )?;

    let component_registry =
        current_registry_for_acceptance(&authority, root, &request, &validation)?;

    let store = root_store::status(component_registry.store_bootstrap.clone()).await?;
    validate_store_artifacts(&store, &validation.component_roles)?;
    let revalidated = current_registry_for_acceptance(&authority, root, &request, &validation)?;
    if revalidated.store_bootstrap != component_registry.store_bootstrap {
        return Err(InternalError::conflict(
            "root Component Registry Store authority changed across acceptance observation",
        ));
    }
    let accepted = RootComponentProvisioningOps::accept(request, &validation, IcOps::now_nanos())?;
    Ok(crate::ops::component_provisioning::status_response(
        accepted,
    ))
}

/// Read one exact durable acceptance receipt under Coordinator authentication.
pub fn status(
    caller: Principal,
    request: RootComponentProvisioningStatusRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, _root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    RootComponentProvisioningOps::status(request)
        .map(crate::ops::component_provisioning::status_response)
}

/// Advance exactly one canonical identity reservation, Canister claim or verified install.
pub async fn advance(
    caller: Principal,
    request: RootComponentProvisioningAdvanceRequest,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    require_coordinator(caller, authority.binding.authority.binding.coordinator)?;
    let current = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &current)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => {
            return Ok(crate::ops::component_provisioning::status_response(current));
        }
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }

    let advanced = if current.reservation_cursor.reserved_component_count < current.component_count
    {
        advance_member_reservation(&authority, root, request, &current)?
    } else if current.claim_cursor.claimed_component_count < current.component_count {
        advance_member_claim(&authority, root, request, &current).await?
    } else {
        Box::pin(advance_member_install(&authority, root, request, &current)).await?
    };
    Ok(crate::ops::component_provisioning::status_response(
        advanced,
    ))
}

fn advance_member_reservation(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    let member = RootComponentProvisioningOps::next_member_reservation(current)?;
    let existing = ComponentRegistryOps::allocation(member.member_operation_id);
    validate_reservation_registry_progress(
        registry.reserved_component_instances,
        current.reservation_cursor.reserved_component_count,
        existing.is_some(),
    )?;
    let topology = ConfigOps::component_topology()?;
    let allocation = match existing {
        Some(allocation) => allocation,
        None => reserve_group_member(authority, &registry, current, &member, &topology)?,
    };
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
}

async fn advance_member_claim(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    validate_claim_registry_progress(&registry, current.component_count)?;
    let member = RootComponentProvisioningOps::next_member_claim(current)?;
    let allocation =
        ComponentRegistryOps::allocation(member.member_operation_id).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Group member claim has no reserved Component identity",
            )
        })?;
    let topology = ConfigOps::component_topology()?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;

    let store = root_store::status(registry.store_bootstrap.clone()).await?;
    let revalidated = current_registry_for_progress(authority, root, current)?;
    if revalidated.store_bootstrap != registry.store_bootstrap {
        return Err(InternalError::conflict(
            "root Component Registry Store authority changed across prepaid-Canister claim observation",
        ));
    }
    let latest = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &latest)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => return Ok(latest),
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }

    let allocation =
        ComponentRegistryOps::allocation(member.member_operation_id).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Group member reservation disappeared across Store observation",
            )
        })?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let claimed =
        super::component_registry::advance_group_member_creation(root, &store, allocation)?;
    let context =
        RootComponentProvisioningOps::member_deployment_context(&latest, &member, &claimed)?;
    validate_group_member_context(&context)?;
    RootComponentProvisioningOps::mark_member_claimed(request, &claimed)
}

async fn advance_member_install(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: RootComponentProvisioningAdvanceRequest,
    current: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningView, InternalError> {
    let registry = current_registry_for_progress(authority, root, current)?;
    validate_claim_registry_progress(&registry, current.component_count)?;
    let member = RootComponentProvisioningOps::next_member_install(current)?;
    let allocation = required_member_allocation(member.member_operation_id, "install")?;
    let topology = ConfigOps::component_topology()?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(current, &member, &allocation)?;
    validate_group_member_context(&deployment)?;

    let store = root_store::status(registry.store_bootstrap.clone()).await?;
    let revalidated = current_registry_for_progress(authority, root, current)?;
    if revalidated.store_bootstrap != registry.store_bootstrap {
        return Err(InternalError::conflict(
            "root Component Registry Store authority changed across grouped install observation",
        ));
    }
    let latest = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &latest)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => return Ok(latest),
        RootComponentProvisioningAdvanceDisposition::Advance => {}
    }
    if RootComponentProvisioningOps::next_member_install(&latest)? != member {
        return Err(InternalError::conflict(
            "root Component provisioning install member changed across Store observation",
        ));
    }

    let allocation = required_member_allocation(member.member_operation_id, "install")?;
    super::component_registry::validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        member.member_operation_id,
    )?;
    let deployment =
        RootComponentProvisioningOps::member_deployment_context(&latest, &member, &allocation)?;
    validate_group_member_context(&deployment)?;
    let installed = Box::pin(super::component_registry::advance_group_member_install(
        &authority.binding,
        &store,
        allocation,
        deployment,
    ))
    .await?;

    let _registry = current_registry_for_progress(authority, root, &latest)?;
    let committed = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })?;
    match RootComponentProvisioningOps::advance_disposition(request, &committed)? {
        RootComponentProvisioningAdvanceDisposition::Complete
        | RootComponentProvisioningAdvanceDisposition::Replay => Ok(committed),
        RootComponentProvisioningAdvanceDisposition::Advance => {
            RootComponentProvisioningOps::mark_member_installed(request, &installed)
        }
    }
}

fn required_member_allocation(
    operation_id: [u8; 32],
    phase: &str,
) -> Result<RootComponentAllocationView, InternalError> {
    ComponentRegistryOps::allocation(operation_id).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            format!("Component Group member {phase} has no reserved Component identity"),
        )
    })
}

fn require_coordinator(caller: Principal, coordinator: Principal) -> Result<(), InternalError> {
    if caller != coordinator {
        return Err(InternalError::forbidden(format!(
            "caller {caller} is not the protected Fleet Coordinator"
        )));
    }
    Ok(())
}

fn current_registry_for_acceptance(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    request: &RootComponentProvisioningAcceptanceRequest,
    validation: &RootComponentProvisioningBatchValidation,
) -> Result<RootComponentRegistryView, InternalError> {
    if FleetActivationWorkflow::status()?.phase != FleetActivationPhase::Prepared {
        return Err(InternalError::conflict(
            "fresh root Component provisioning acceptance requires runtime Prepared",
        ));
    }
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != request.fleet_registry
    {
        return Err(InternalError::conflict(
            "root Component provisioning request differs from the exact active Registry mirror",
        ));
    }
    let current = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    validate_component_registry_authority(
        &current,
        &authority.binding,
        authority.initial_release_set,
        &request.fleet_registry,
    )?;
    validate_component_capacity(&current, validation)?;
    validate_group_placement_capacity(
        RootComponentProvisioningOps::tracked_group_placements()?,
        validation.placement_count,
        authority.binding.limits.maximum_group_placements,
    )?;
    validate_ready_pool_capacity(validation.component_count)?;
    Ok(current)
}

fn current_registry_for_progress(
    authority: &FleetSubnetRootAuthority,
    root: Principal,
    provisioning: &RootComponentProvisioningView,
) -> Result<RootComponentRegistryView, InternalError> {
    if FleetActivationWorkflow::status()?.phase != FleetActivationPhase::Prepared {
        return Err(InternalError::conflict(
            "fresh root Component provisioning requires runtime Prepared",
        ));
    }
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status != FleetSubnetRootStatus::Active
        || mirror.active.snapshot.version != provisioning.fleet_registry
    {
        return Err(InternalError::conflict(
            "root Component provisioning differs from the exact active Registry mirror",
        ));
    }
    let config = ConfigOps::get()?;
    ComponentProvisioningPlanOps::validate_root_batch(
        &config,
        &mirror.active.snapshot.registry,
        &provisioning.fleet_registry,
        provisioning.configuration_digest,
        &authority.binding,
        &provisioning.batch,
    )?;
    let current = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    validate_component_registry_authority(
        &current,
        &authority.binding,
        authority.initial_release_set,
        &provisioning.fleet_registry,
    )?;
    Ok(current)
}

fn validate_reservation_registry_progress(
    registry_reserved_components: u32,
    aggregate_reserved_components: u32,
    current_member_exists: bool,
) -> Result<(), InternalError> {
    let expected = aggregate_reserved_components
        .checked_add(u32::from(current_member_exists))
        .ok_or_else(|| {
            InternalError::resource_exhausted(
                "root Component provisioning reservation count overflowed",
            )
        })?;
    if registry_reserved_components != expected {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry reservations differ from aggregate provisioning progress",
        ));
    }
    Ok(())
}

fn validate_claim_registry_progress(
    registry: &RootComponentRegistryView,
    component_count: u32,
) -> Result<(), InternalError> {
    if registry.reserved_component_instances != component_count {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry reservations differ from claim-ready aggregate provisioning progress",
        ));
    }
    Ok(())
}

fn validate_group_member_context(
    context: &canic_core::dto::component_deployment::ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    let canic_core::dto::component_deployment::ProtectedComponentDeployment::GroupMember {
        binding,
        ..
    } = context
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "group provisioning derived an ordinary Component deployment context",
        ));
    };
    ConfigOps::validate_protected_component_deployment(context, binding)
}

fn reserve_group_member(
    authority: &FleetSubnetRootAuthority,
    registry: &RootComponentRegistryView,
    provisioning: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
) -> Result<RootComponentAllocationView, InternalError> {
    let request = RootComponentAllocationRequest {
        operation_id: member.member_operation_id,
        component_spec: member.component_spec.clone(),
    };
    let decision = super::component_registry::top_level_allocation_decision(
        &authority.binding,
        topology,
        registry,
        &request,
    )?;
    let origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: provisioning.operation_id,
        plan_hash: provisioning.plan_hash,
        group_placement: member.group_placement.clone(),
        member_path: member.member_path.clone(),
    };
    ComponentRegistryOps::reserve_allocation(decision, member.member_operation_id, origin, false)
}

fn validate_component_registry_authority(
    current: &RootComponentRegistryView,
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    fleet_registry: &canic_core::dto::fleet_registry::FleetRegistryVersion,
) -> Result<(), InternalError> {
    if &current.root != root
        || current.release_set != release_set
        || current.initial_inventory.is_some()
        || current.root_draining.is_some()
        || !ComponentRegistryOps::registry_covers_preparation(
            &current.prepared_against_registry,
            fleet_registry,
        )
    {
        return Err(InternalError::conflict(
            "root Component provisioning authority differs from the open prepared Component Registry",
        ));
    }
    ComponentRegistryOps::require_top_level_allocation_open()
}

fn validate_component_capacity(
    current: &RootComponentRegistryView,
    validation: &RootComponentProvisioningBatchValidation,
) -> Result<(), InternalError> {
    if current.reserved_component_instances != 0 {
        return Err(InternalError::unavailable(
            "root has nonterminal top-level Component allocations",
        ));
    }
    let occupied = current
        .reserved_component_instances
        .checked_add(current.committed_component_instances)
        .and_then(|count| count.checked_add(validation.component_count))
        .ok_or_else(|| {
            InternalError::resource_exhausted("root Component instance accounting overflowed")
        })?;
    if occupied > current.root.limits.maximum_component_instances {
        return Err(InternalError::resource_exhausted(format!(
            "root provisioning batch requires {occupied} Component instances, exceeding protected limit {}",
            current.root.limits.maximum_component_instances
        )));
    }
    for (component_spec, requested) in &validation.component_spec_counts {
        let admission = current
            .root
            .component_admissions
            .binary_search_by(|candidate| candidate.component_spec.cmp(component_spec))
            .ok()
            .map(|index| &current.root.component_admissions[index])
            .ok_or_else(|| {
                InternalError::conflict(format!(
                    "root has no admission for planned Component Spec '{component_spec}'"
                ))
            })?;
        let counts = ComponentRegistryOps::component_spec_counts(component_spec)?;
        let occupied = counts
            .reserved
            .checked_add(counts.committed)
            .and_then(|count| count.checked_add(*requested))
            .ok_or_else(|| {
                InternalError::resource_exhausted(
                    "root Component Spec instance accounting overflowed",
                )
            })?;
        if occupied > admission.maximum_root_instances {
            return Err(InternalError::resource_exhausted(format!(
                "root provisioning batch requires {occupied} instances of Component Spec '{component_spec}', exceeding admission {}",
                admission.maximum_root_instances
            )));
        }
    }
    Ok(())
}

fn validate_group_placement_capacity(
    tracked: u32,
    requested: u32,
    maximum: u32,
) -> Result<(), InternalError> {
    let required = tracked.checked_add(requested).ok_or_else(|| {
        InternalError::resource_exhausted("root Component Group placement accounting overflowed")
    })?;
    if required > maximum {
        return Err(InternalError::resource_exhausted(format!(
            "root provisioning batch requires {required} group placements, exceeding protected limit {maximum}"
        )));
    }
    Ok(())
}

fn validate_ready_pool_capacity(component_count: u32) -> Result<(), InternalError> {
    let ready = CanisterPoolOps::ready_count();
    if ready < component_count {
        return Err(InternalError::unavailable(format!(
            "root provisioning batch requires {component_count} Ready prepaid Canisters but only {ready} are available"
        )));
    }
    Ok(())
}

fn validate_store_artifacts(
    store: &canic_core::dto::root_store::RootStoreBootstrapResponse,
    roles: &std::collections::BTreeSet<canic_core::ids::CanisterRole>,
) -> Result<(), InternalError> {
    for role in roles {
        let count = store
            .catalog
            .iter()
            .filter(|artifact| &artifact.role == role)
            .count();
        if count != 1 {
            return Err(InternalError::conflict(format!(
                "root Wasm Store Catalog has {count} artifacts for planned Component role '{role}'"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_protected_coordinator_is_authorized() {
        let coordinator = Principal::from_slice(&[7; 29]);
        assert!(require_coordinator(coordinator, coordinator).is_ok());
        assert!(require_coordinator(Principal::from_slice(&[8; 29]), coordinator).is_err());
    }

    #[test]
    fn capacity_helpers_reject_first_excess_without_mutation() {
        assert!(validate_group_placement_capacity(3, 2, 5).is_ok());
        assert!(validate_group_placement_capacity(3, 3, 5).is_err());
    }

    #[test]
    fn registry_progress_allows_only_exact_or_response_lost_reservation() {
        assert!(validate_reservation_registry_progress(3, 3, false).is_ok());
        assert!(validate_reservation_registry_progress(4, 3, true).is_ok());
        assert!(validate_reservation_registry_progress(4, 3, false).is_err());
        assert!(validate_reservation_registry_progress(3, 3, true).is_err());
    }
}
