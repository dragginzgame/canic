//! Module: fleet_ensure::ops::current_inventory::ordinary
//!
//! Responsibility: join ordinary allocations to exact Root partitions and pool claims.
//! Does not own: Root provisioning authority or provisioning effects.
//! Boundary: observations require current Registry, release and live canister authority.

use super::{
    ComponentBinding, ComponentInstanceId, ComponentLifecycleStatus, ComponentProvisioningOrigin,
    ComponentRegistryPartitionResponse, ComponentTopology, CurrentProtocolError, DescendantParent,
    FleetRegistry, FleetSubnetRootEntry, IcpCli, Principal, ProtocolCatalog, ProtocolEntry,
    RegistryEntry, RoleCapabilityKey, RootComponentAllocationPhase, TerminalWorkloadAuthority,
    inspect_root_controlled_canister, inventory_error, observed_cycle_balance, query_with_candid,
    require_terminal_component_authority, terminal_field_exact, terminal_nonzero_hash,
    terminal_observation,
};
use candid::CandidType;
use canic_control_plane::dto::root::{RootComponentOperationStatus, RootOperationStatusResponse};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{component_registry::ComponentRegistryPartitionRequest, role::OperationStatusRequest},
    protocol,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(CandidType)]
enum Request {
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ComponentRegistryPartition(Box<ComponentRegistryPartitionResponse>),
    Operation(Box<RootOperationStatusResponse>),
}

///
/// OrdinaryComponentObservation
///
/// Ops-owned contribution of one qualified ordinary Component to terminal inventory.
///

pub(super) struct OrdinaryComponentObservation {
    pub binding: ComponentBinding,
    pub cycles: u128,
    pub entry: RegistryEntry,
    pub parent: Option<DescendantParent>,
    pub workload: TerminalWorkloadAuthority,
}

/// Discover missing top-level owners from bounded Root pool claims, preserving descendant joins.
#[expect(
    clippy::too_many_lines,
    reason = "one bounded observation joins current Registry, protocol, partition and allocation authority"
)]
pub(super) fn observe(
    icp: &IcpCli,
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    protocols: &ProtocolCatalog,
    topology: &ComponentTopology,
    known: &BTreeMap<ComponentInstanceId, ComponentBinding>,
    pool: &BTreeMap<Principal, TerminalWorkloadAuthority>,
) -> Result<Vec<OrdinaryComponentObservation>, CurrentProtocolError> {
    let candidates = pool
        .values()
        .filter(|claim| {
            claim.root == root.fleet_subnet_root && !known.contains_key(&claim.component)
        })
        .map(|claim| claim.component)
        .collect::<BTreeSet<_>>();
    let existing = known
        .values()
        .filter(|binding| binding.fleet_subnet_root == root.fleet_subnet_root)
        .collect::<Vec<_>>();
    if existing.len().saturating_add(candidates.len())
        > root.limits.maximum_component_instances as usize
    {
        return Err(inventory_error(
            "ordinary Components exceed Root instance capacity",
        ));
    }
    let mut counts = BTreeMap::new();
    for binding in existing {
        *counts
            .entry(binding.component_spec.clone())
            .or_insert(0_u32) += 1;
    }
    let mut output = Vec::with_capacity(candidates.len());
    for component in candidates {
        let Response::ComponentRegistryPartition(partition) = read(
            icp,
            &protocols.root.candid_path,
            root.fleet_subnet_root,
            &Request::ComponentRegistryPartition(ComponentRegistryPartitionRequest { component }),
        )?
        else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        terminal_field_exact(
            "ordinary.component",
            &component,
            &partition.binding.component,
        )?;
        let binding = &partition.binding;
        let workload = pool
            .get(&binding.canister_id)
            .ok_or_else(|| inventory_error("ordinary Component has no top-level pool claim"))?;
        let Response::Operation(operation) = read(
            icp,
            &protocols.root.candid_path,
            root.fleet_subnet_root,
            &Request::Operation(OperationStatusRequest {
                operation_id: workload.operation_id,
            }),
        )?
        else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        let RootOperationStatusResponse::ProvisionComponent(operation) = *operation else {
            return Err(inventory_error(
                "ordinary Component pool claim names another operation kind",
            ));
        };
        let protocol = protocols
            .child(&binding.role)
            .ok_or_else(|| inventory_error("ordinary Component has no current role protocol"))?;
        validate(registry, root, workload, &partition, &operation, protocol)?;
        let admission = root
            .component_admissions
            .iter()
            .find(|entry| entry.component_spec == binding.component_spec)
            .ok_or_else(|| inventory_error("ordinary Component Spec is not admitted"))?;
        let count = counts
            .entry(binding.component_spec.clone())
            .or_insert(0_u32);
        *count = count
            .checked_add(1)
            .ok_or_else(|| inventory_error("ordinary Component count overflowed"))?;
        if *count > admission.maximum_root_instances {
            return Err(inventory_error(
                "ordinary Components exceed Root Spec admission",
            ));
        }
        let spec = topology.get(&binding.component_spec).ok_or_else(|| {
            inventory_error("ordinary Component Spec is absent from current configuration")
        })?;
        let descendants =
            u64::from(partition.reserved_descendants) + u64::from(partition.committed_descendants);
        if descendants > u64::from(spec.limits.maximum_descendants)
            || partition.encoded_bytes > spec.limits.maximum_registry_bytes
        {
            return Err(inventory_error(
                "ordinary Component partition exceeds Spec capacity",
            ));
        }
        let observed = inspect_root_controlled_canister(
            icp,
            &protocols.root.candid_path,
            root.fleet_subnet_root,
            binding.canister_id,
        )?;
        require_terminal_component_authority(
            root.fleet_subnet_root,
            binding.canister_id,
            &observed,
            protocol,
        )?;
        output.push(OrdinaryComponentObservation {
            binding: binding.clone(),
            cycles: observed_cycle_balance(&observed)?,
            entry: RegistryEntry {
                pid: binding.canister_id.to_text(),
                role: Some(binding.role.to_string()),
                parent_pid: Some(root.fleet_subnet_root.to_text()),
                module_hash: Some(protocol.installed_module_hash.clone()),
                protocol_binding: Some(protocol.binding.clone()),
            },
            parent: protocol
                .binding
                .capabilities
                .contains(&RoleCapabilityKey::ChildProvisioning)
                .then(|| DescendantParent {
                    canister_id: binding.canister_id,
                    candid_path: protocol.candid_path.clone(),
                    maximum_children: u64::from(spec.limits.maximum_descendants),
                    release_set: root.active_release_set,
                    role: binding.role.clone(),
                    root: root.fleet_subnet_root,
                }),
            workload: workload.clone(),
        });
    }
    Ok(output)
}

fn read(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    request: &Request,
) -> Result<Response, CurrentProtocolError> {
    let method = match request {
        Request::ComponentRegistryPartition(_) => protocol::CANIC_ROOT_STATUS,
        Request::Operation(_) => protocol::CANIC_ROOT_OPERATION_STATUS,
    };
    terminal_observation(
        "ordinary_component",
        query_with_candid(icp, candid, root, method, request),
    )
}

pub(super) fn validate(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    workload: &TerminalWorkloadAuthority,
    partition: &ComponentRegistryPartitionResponse,
    operation: &RootComponentOperationStatus,
    protocol: &ProtocolEntry,
) -> Result<(), CurrentProtocolError> {
    let binding = &partition.binding;
    let spec = registry
        .component_specs
        .iter()
        .find(|entry| entry.component_spec == binding.component_spec)
        .ok_or_else(|| inventory_error("ordinary Component names an unknown Spec"))?;
    if matches!(
        partition.provisioning_origin,
        ComponentProvisioningOrigin::ComponentGroup { .. }
    ) {
        return Err(inventory_error(
            "an unaccounted Component Group member is not an ordinary Component",
        ));
    }
    terminal_field_exact(
        "ordinary.authority",
        &registry.authority,
        &binding.authority,
    )?;
    terminal_field_exact(
        "ordinary.root",
        &root.fleet_subnet_root,
        &binding.fleet_subnet_root,
    )?;
    terminal_field_exact(
        "ordinary.subnet",
        &root.placement_subnet,
        &binding.placement_subnet,
    )?;
    terminal_field_exact("ordinary.spec_hash", &spec.spec_hash, &binding.spec_hash)?;
    terminal_field_exact("ordinary.role", &spec.component_role, &binding.role)?;
    let admission = root
        .component_admissions
        .iter()
        .find(|entry| entry.component_spec == binding.component_spec)
        .ok_or_else(|| inventory_error("ordinary Component Spec is not admitted"))?;
    terminal_field_exact(
        "ordinary.admission",
        &binding.spec_hash,
        &admission.spec_hash,
    )?;
    validate_allocation(root, workload, partition, operation, protocol)?;
    terminal_field_exact(
        "ordinary.status",
        &ComponentLifecycleStatus::Active,
        &partition.status,
    )?;
    terminal_field_exact(
        "ordinary.head",
        &binding.component,
        &partition.head.component,
    )?;
    terminal_nonzero_hash("ordinary.partition_hash", partition.head.content_hash)?;
    terminal_field_exact(
        "ordinary.protocol",
        &protocol.binding.protocol_profile_digest,
        &partition.protocol_profile_digest,
    )
}

fn validate_allocation(
    root: &FleetSubnetRootEntry,
    workload: &TerminalWorkloadAuthority,
    partition: &ComponentRegistryPartitionResponse,
    operation: &RootComponentOperationStatus,
    protocol: &ProtocolEntry,
) -> Result<(), CurrentProtocolError> {
    let binding = &partition.binding;
    let allocation = &operation.allocation;
    let installation = allocation
        .installation
        .as_ref()
        .ok_or_else(|| inventory_error("ordinary Component lacks installation evidence"))?;
    terminal_field_exact(
        "ordinary.pool_root",
        &binding.fleet_subnet_root,
        &workload.root,
    )?;
    terminal_field_exact(
        "ordinary.pool_component",
        &binding.component,
        &workload.component,
    )?;
    terminal_field_exact(
        "ordinary.operation",
        &workload.operation_id,
        &allocation.operation_id,
    )?;
    terminal_field_exact(
        "ordinary.allocation_component",
        &binding.component,
        &allocation.component,
    )?;
    terminal_field_exact(
        "ordinary.allocation_spec",
        &binding.component_spec,
        &allocation.component_spec,
    )?;
    terminal_field_exact(
        "ordinary.allocation_spec_hash",
        &binding.spec_hash,
        &allocation.spec_hash,
    )?;
    terminal_field_exact("ordinary.allocation_role", &binding.role, &allocation.role)?;
    terminal_field_exact("ordinary.installation", binding, &installation.binding)?;
    terminal_field_exact(
        "ordinary.raw_module",
        &protocol.raw_module_hash,
        &hex_bytes(installation.raw_module_hash),
    )?;
    terminal_field_exact(
        "ordinary.origin",
        &partition.provisioning_origin,
        &allocation.provisioning_origin,
    )?;
    terminal_field_exact(
        "ordinary.release",
        &root.active_release_set,
        &allocation.release_set,
    )?;
    terminal_field_exact(
        "ordinary.partition_release",
        &root.active_release_set,
        &partition.release_set,
    )?;
    terminal_field_exact(
        "ordinary.phase",
        &RootComponentAllocationPhase::Committed,
        &allocation.phase,
    )?;
    terminal_field_exact("ordinary.complete", &true, &operation.complete)?;
    Ok(())
}
