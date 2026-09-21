//! Module: fleet_ensure::ops::startup_funding::observation::inventory
//!
//! Responsibility: join funding allocations to complete bounded current Component directories.
//! Boundary: query-only membership evidence; it neither collects paid ledgers nor approves funding.

mod pages;
#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::query_with_candid,
    fleet_ensure::view::startup_funding::{
        StartupChildFundingBinding, StartupFundingPlacement, StartupInventoryCoverage,
        StartupRootInventory, StartupUsageUnavailable,
    },
    icp::IcpCli,
};
use candid::{CandidType, Deserialize, Principal};
use canic_core::{
    control_plane_support::config::ComponentTopology,
    dto::component_registry::{
        ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentLifecycleStatus,
        ComponentRegistryPartitionRequest, ComponentRegistryPartitionResponse,
    },
    ids::{ComponentBinding, ComponentInstanceId, ComponentSpecId, FleetSubnetRootReleaseSet},
    protocol,
};
use std::{collections::BTreeMap, path::Path};

#[derive(CandidType)]
enum Request {
    ComponentDirectoryHead(ComponentDirectoryHeadRequest),
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ComponentDirectoryHead(Box<ComponentDirectoryHead>),
    ComponentRegistryPartition(Box<ComponentRegistryPartitionResponse>),
}

/// Invocation-local evidence retained across ledger reads, with no durable spending authority.
pub struct FundingInventoryObservation {
    components: Vec<ComponentInventory>,
    coverage: StartupInventoryCoverage,
}

struct ComponentInventory {
    partition: ComponentRegistryPartitionResponse,
    directory: ComponentDirectoryHead,
}

#[derive(Eq, PartialEq)]
struct PartitionAuthority<'a> {
    binding: &'a ComponentBinding,
    release: FleetSubnetRootReleaseSet,
    component: ComponentInstanceId,
}

/// Require the complete observed Workload set to appear in current Root-owned directories.
pub fn observe(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    placement: &StartupFundingPlacement,
    topology: &ComponentTopology,
    summary: StartupRootInventory,
    bindings: &[&StartupChildFundingBinding],
) -> Result<FundingInventoryObservation, StartupUsageUnavailable> {
    let groups = group(placement, summary, bindings)?;
    let mut components = Vec::with_capacity(groups.len());
    for members in groups.values() {
        let component = &members[0].component;
        let partition = partition(icp, candid, root, component.component)?;
        validate_partition(&partition, topology, members)?;
        let directory = directory(icp, candid, root, component.component)?;
        validate_directory(root, &partition, &directory)?;
        pages::observe(icp, candid, root, &directory, members)?;
        components.push(ComponentInventory {
            partition,
            directory,
        });
    }
    Ok(FundingInventoryObservation {
        coverage: StartupInventoryCoverage {
            components: components.len(),
            descendants: bindings.len() - components.len(),
        },
        components,
    })
}

/// Recheck every independent Component head after ledger reads, without replaying page reads.
pub fn recheck(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    evidence: &FundingInventoryObservation,
) -> Result<StartupInventoryCoverage, StartupUsageUnavailable> {
    for component in &evidence.components {
        let id = component.partition.binding.component;
        if partition(icp, candid, root, id)? != component.partition
            || directory(icp, candid, root, id)? != component.directory
        {
            return Err(StartupUsageUnavailable::PolicyTransition);
        }
    }
    Ok(evidence.coverage)
}

/// Preserve the exact heads checked across the collection in an observation review.
pub(in crate::fleet_ensure) fn heads(
    evidence: &FundingInventoryObservation,
) -> BTreeMap<
    ComponentInstanceId,
    crate::fleet_ensure::model::funding_observation::FundingComponentHeadRecord,
> {
    evidence
        .components
        .iter()
        .map(|entry| {
            (
                entry.partition.binding.component,
                crate::fleet_ensure::model::funding_observation::FundingComponentHeadRecord {
                    revision: entry.partition.head.revision,
                    content_hash: entry.partition.head.content_hash,
                },
            )
        })
        .collect()
}

fn group<'a>(
    placement: &StartupFundingPlacement,
    summary: StartupRootInventory,
    bindings: &[&'a StartupChildFundingBinding],
) -> Result<
    BTreeMap<ComponentInstanceId, Vec<&'a StartupChildFundingBinding>>,
    StartupUsageUnavailable,
> {
    if u64::from(summary.workloads) != bindings.len() as u64 {
        return Err(StartupUsageUnavailable::InventoryIncomplete);
    }
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    let mut seen = std::collections::BTreeSet::new();
    for binding in bindings {
        if !seen.insert(binding.canister_id) {
            return Err(StartupUsageUnavailable::AuthorityMismatch);
        }
        groups
            .entry(binding.component.component)
            .or_default()
            .push(*binding);
    }
    if groups.len() > placement.limits.maximum_component_instances as usize {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    let mut counts = BTreeMap::<&ComponentSpecId, usize>::new();
    for members in groups.values() {
        let component = &members[0].component;
        let top_count = members
            .iter()
            .filter(|member| member.canister_id == component.canister_id)
            .count();
        if top_count != 1 {
            return Err(StartupUsageUnavailable::InventoryIncomplete);
        }
        if members.iter().any(|member| member.component != *component) {
            return Err(StartupUsageUnavailable::AuthorityMismatch);
        }
        let count = counts.entry(&component.component_spec).or_default();
        *count += 1;
        let admission = placement
            .component_admissions
            .iter()
            .find(|admission| admission.component_spec == component.component_spec)
            .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
        if *count > admission.maximum_root_instances as usize {
            return Err(StartupUsageUnavailable::AuthorityMismatch);
        }
    }
    Ok(groups)
}

fn partition(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    component: ComponentInstanceId,
) -> Result<ComponentRegistryPartitionResponse, StartupUsageUnavailable> {
    let response = query_with_candid(
        icp,
        candid,
        root,
        protocol::CANIC_ROOT_STATUS,
        &Request::ComponentRegistryPartition(ComponentRegistryPartitionRequest { component }),
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    let Response::ComponentRegistryPartition(partition) = response else {
        return Err(StartupUsageUnavailable::ObservationFailed);
    };
    Ok(*partition)
}

fn directory(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    component: ComponentInstanceId,
) -> Result<ComponentDirectoryHead, StartupUsageUnavailable> {
    let response = query_with_candid(
        icp,
        candid,
        root,
        protocol::CANIC_ROOT_STATUS,
        &Request::ComponentDirectoryHead(ComponentDirectoryHeadRequest { component }),
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    let Response::ComponentDirectoryHead(directory) = response else {
        return Err(StartupUsageUnavailable::ObservationFailed);
    };
    Ok(*directory)
}

fn validate_partition(
    partition: &ComponentRegistryPartitionResponse,
    topology: &ComponentTopology,
    members: &[&StartupChildFundingBinding],
) -> Result<(), StartupUsageUnavailable> {
    let binding = members[0];
    let actual = PartitionAuthority {
        binding: &partition.binding,
        release: partition.release_set,
        component: partition.head.component,
    };
    let expected = PartitionAuthority {
        binding: &binding.component,
        release: binding.release_set,
        component: binding.component.component,
    };
    if actual != expected {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    if partition.status != ComponentLifecycleStatus::Active || partition.reserved_descendants != 0 {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    let spec = topology
        .get(&binding.component.component_spec)
        .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
    if partition.head.revision == 0
        || partition.head.content_hash == [0; 32]
        || partition.committed_descendants > spec.limits.maximum_descendants
        || partition.encoded_bytes > spec.limits.maximum_registry_bytes
    {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    if u64::from(partition.committed_descendants) + 1 != members.len() as u64 {
        return Err(StartupUsageUnavailable::InventoryIncomplete);
    }
    Ok(())
}

fn validate_directory(
    root: Principal,
    partition: &ComponentRegistryPartitionResponse,
    directory: &ComponentDirectoryHead,
) -> Result<(), StartupUsageUnavailable> {
    let provenance = &directory.provenance;
    if provenance.component != partition.binding || provenance.source_fleet_subnet_root != root {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    let current = (
        partition.head.revision,
        partition.head.content_hash,
        partition.committed_descendants,
    );
    let observed = (
        provenance.component_registry_revision,
        provenance.component_registry_content_hash,
        directory.descendant_count,
    );
    if observed != current {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    Ok(())
}
