//! Module: ops::fleet_coordinator::root_lifecycle
//!
//! Responsibility: validate Coordinator-owned Root draining and removal publications.
//! Does not own: durable Registry storage, endpoint authorization, or Root-side effects.
//! Boundary: binds each lifecycle publication to exact Registry, reservation, and Root authority.

use super::registry_history::FleetRegistryHistoryPoint;
use crate::storage::stable::fleet_coordinator::{
    FleetCoordinatorRegistryRecord, FleetSubnetRootDrainingPublicationReceiptRecord,
    FleetSubnetRootRemovalPublicationReceiptRecord,
};
use candid::Principal;
use canic_core::{
    control_plane_support::error::InternalError,
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryVersion, FleetSubnetRootDrainingPublicationRequest,
        FleetSubnetRootDrainingReservationRequest, FleetSubnetRootDrainingReservationResponse,
        FleetSubnetRootDrainingReservationStatusRequest, FleetSubnetRootEntry,
        FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootStatus,
    },
    ids::{ComponentTopologyDigest, FleetSubnetRootReleaseSet, SubnetId},
};

#[derive(Eq, PartialEq)]
struct FleetSubnetRootDrainingAuthority {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl FleetSubnetRootDrainingAuthority {
    const fn from_registry(entry: &FleetSubnetRootEntry) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &FleetSubnetRootDrainingPublicationRequest) -> Self {
        let draining = &request.root_draining;
        Self {
            fleet_subnet_root: draining.fleet_subnet_root,
            placement_subnet: draining.placement_subnet,
            component_topology_digest: draining.component_topology_digest,
            active_release_set: draining.active_release_set,
        }
    }
}

pub(super) fn draining_publication_identity_matches(
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_publication_request(&receipt.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_publication_request(request),
    )
}

pub(super) fn draining_reservation_for_publication<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> Result<&'a FleetSubnetRootDrainingReservationResponse, InternalError> {
    let publication_identity = FleetSubnetRootDrainingIdentity::from_publication_request(request);
    let reservation = current
        .root_draining_reservations
        .iter()
        .find(|record| {
            FleetSubnetRootDrainingIdentity::from_reservation_request(&record.response.request)
                .conflicts_with(publication_identity)
        })
        .ok_or_else(InternalError::unavailable)?;
    let reservation_identity =
        FleetSubnetRootDrainingIdentity::from_reservation_request(&reservation.response.request);
    if reservation_identity != publication_identity {
        return Err(InternalError::conflict());
    }
    Ok(&reservation.response)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct FleetSubnetRootDrainingIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootDrainingIdentity {
    pub(super) const fn from_publication_request(
        request: &FleetSubnetRootDrainingPublicationRequest,
    ) -> Self {
        Self {
            fleet_subnet_root: request.root_draining.fleet_subnet_root,
            operation_id: request.root_draining.operation_id,
        }
    }

    pub(super) const fn from_reservation_request(
        request: &FleetSubnetRootDrainingReservationRequest,
    ) -> Self {
        Self {
            fleet_subnet_root: request.expected_root.fleet_subnet_root,
            operation_id: request.operation_id,
        }
    }

    const fn from_reservation_status(
        request: &FleetSubnetRootDrainingReservationStatusRequest,
    ) -> Self {
        Self {
            fleet_subnet_root: request.fleet_subnet_root,
            operation_id: request.operation_id,
        }
    }

    pub(super) fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

pub(super) fn draining_reservation_identity_matches(
    response: &FleetSubnetRootDrainingReservationResponse,
    request: &FleetSubnetRootDrainingReservationRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_reservation_request(&response.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_reservation_request(request),
    )
}

pub(super) fn draining_reservation_status_matches(
    response: &FleetSubnetRootDrainingReservationResponse,
    request: &FleetSubnetRootDrainingReservationStatusRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_reservation_request(&response.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_reservation_status(request),
    )
}

pub(super) fn validate_root_draining_reservation_request(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingReservationRequest,
) -> Result<(), InternalError> {
    if request.expected_registry != *version {
        return Err(InternalError::conflict());
    }
    if request.expected_root.status != FleetSubnetRootStatus::Active {
        return Err(InternalError::invalid_input());
    }
    let Some(target) = current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == request.expected_root.fleet_subnet_root)
    else {
        return Err(InternalError::conflict());
    };
    if target != &request.expected_root {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) fn validate_draining_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingPublicationRequest,
    reservation: &FleetSubnetRootDrainingReservationResponse,
) -> Result<(), &'static str> {
    let draining = &request.root_draining;
    if request.expected_registry != *version {
        return Err("Fleet Subnet Root draining publication names stale Registry authority");
    }
    let reservation_matches_receipt = [
        reservation.request.operation_id == draining.operation_id,
        reservation.request.expected_registry == draining.active_registry,
        reservation.request.expected_root.fleet_subnet_root == draining.fleet_subnet_root,
        reservation.request.expected_root.status == FleetSubnetRootStatus::Active,
        reservation.reservation_hash != [0; 32],
        reservation.reservation_hash == draining.reservation_hash,
    ]
    .into_iter()
    .all(|valid| valid);
    if !reservation_matches_receipt {
        return Err("Fleet Subnet Root draining receipt differs from its retained reservation");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == draining.fleet_subnet_root)
        .ok_or("Fleet Subnet Root draining publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Active {
        return Err("Fleet Subnet Root draining publication target is not Active");
    }
    if target != &reservation.request.expected_root {
        return Err("Fleet Subnet Root draining reservation differs from current root authority");
    }
    let expected_authority = FleetSubnetRootDrainingAuthority::from_registry(target);
    if FleetSubnetRootDrainingAuthority::from_publication(request) != expected_authority {
        return Err("Fleet Subnet Root draining receipt differs from Registry root authority");
    }
    if draining.operation_id == [0; 32]
        || draining.started_at_ns == 0
        || draining.next_allocation_sequence == 0
    {
        return Err("Fleet Subnet Root draining receipt contains non-positive operation facts");
    }
    let component_instances = draining
        .reserved_component_instances
        .checked_add(draining.committed_component_instances)
        .ok_or("Fleet Subnet Root draining Component Instance count overflowed")?;
    if component_instances > target.limits.maximum_component_instances {
        return Err("Fleet Subnet Root draining Component Instance count exceeds its limit");
    }
    if draining.next_allocation_sequence <= u64::from(component_instances) {
        return Err("Fleet Subnet Root draining allocation sequence precedes its live instances");
    }
    let allocated_canisters = component_instances
        .checked_add(draining.managed_descendants)
        .ok_or("Fleet Subnet Root draining managed canister count overflowed")?;
    if draining.known_created_component_canisters > allocated_canisters {
        return Err("Fleet Subnet Root draining created canisters exceed allocated canisters");
    }
    if draining.root_registry_encoded_bytes > target.limits.maximum_registry_bytes {
        return Err("Fleet Subnet Root draining Registry bytes exceed the root limit");
    }
    Ok(())
}

pub(super) fn removal_publication_identity_matches(
    receipt: &FleetSubnetRootRemovalPublicationReceiptRecord,
    request: &FleetSubnetRootRemovalPublicationRequest,
) -> bool {
    FleetSubnetRootRemovalPublicationIdentity::from_request(&receipt.request).conflicts_with(
        FleetSubnetRootRemovalPublicationIdentity::from_request(request),
    )
}

#[derive(Clone, Copy)]
pub(super) struct FleetSubnetRootRemovalPublicationIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootRemovalPublicationIdentity {
    pub(super) const fn from_request(request: &FleetSubnetRootRemovalPublicationRequest) -> Self {
        Self {
            fleet_subnet_root: request.final_inventory.fleet_subnet_root,
            operation_id: request.final_inventory.operation_id,
        }
    }

    pub(super) fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

#[derive(Eq, PartialEq)]
struct FleetSubnetRootFinalInventoryAuthority {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl FleetSubnetRootFinalInventoryAuthority {
    const fn from_registry(entry: &FleetSubnetRootEntry) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &FleetSubnetRootRemovalPublicationRequest) -> Self {
        let inventory = &request.final_inventory;
        Self {
            fleet_subnet_root: inventory.fleet_subnet_root,
            placement_subnet: inventory.placement_subnet,
            component_topology_digest: inventory.component_topology_digest,
            active_release_set: inventory.active_release_set,
        }
    }
}

pub(super) fn validate_removal_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    draining_receipts: &[FleetSubnetRootDrainingPublicationReceiptRecord],
    history: &[FleetRegistryHistoryPoint],
    request: &FleetSubnetRootRemovalPublicationRequest,
) -> Result<(), &'static str> {
    let inventory = &request.final_inventory;
    if request.expected_registry != *version {
        return Err("Fleet Subnet Root removal publication names stale Registry authority");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == inventory.fleet_subnet_root)
        .ok_or("Fleet Subnet Root removal publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Draining {
        return Err("Fleet Subnet Root removal publication target is not Draining");
    }
    if FleetSubnetRootFinalInventoryAuthority::from_publication(request)
        != FleetSubnetRootFinalInventoryAuthority::from_registry(target)
    {
        return Err("Fleet Subnet Root final inventory differs from Registry root authority");
    }
    let draining = draining_receipts
        .iter()
        .find(|receipt| {
            receipt.request.root_draining.fleet_subnet_root == inventory.fleet_subnet_root
                && receipt.request.root_draining.operation_id == inventory.operation_id
        })
        .ok_or("Fleet Subnet Root final inventory lacks its draining publication")?;
    if inventory.finalized_at_ns < draining.request.root_draining.started_at_ns {
        return Err("Fleet Subnet Root final inventory predates its draining publication");
    }
    let source = history
        .iter()
        .find(|point| point.version == inventory.registry)
        .ok_or("Fleet Subnet Root final inventory Registry is not canonical history")?;
    let source_is_draining = source.registry.fleet_subnet_roots.iter().any(|entry| {
        entry.fleet_subnet_root == inventory.fleet_subnet_root
            && entry.status == FleetSubnetRootStatus::Draining
    });
    if !source_is_draining {
        return Err("Fleet Subnet Root was not Draining at its final inventory Registry");
    }
    let removed_instances_are_exact = u64::from(inventory.removed_component_instances)
        == inventory.next_allocation_sequence.saturating_sub(1);
    let expected_store_entries = inventory
        .wasm_store_catalog_entries
        .checked_add(1)
        .ok_or("Fleet Subnet Root final inventory Store count overflows")?;
    let terminal_facts_are_exact = [
        inventory.operation_id != [0; 32],
        inventory.next_allocation_sequence > 0,
        removed_instances_are_exact,
        inventory.terminal_component_history_hash != [0; 32],
        inventory.root_registry_encoded_bytes <= target.limits.maximum_registry_bytes,
        inventory.wasm_store != Principal::anonymous(),
        inventory.wasm_store_catalog_hash != [0; 32],
        inventory.wasm_store_catalog_entries > 0,
        inventory.wasm_store_release_count == expected_store_entries,
        inventory.wasm_store_template_count == expected_store_entries,
        inventory.wasm_store_occupied_bytes <= target.limits.maximum_wasm_store_bytes,
        inventory.wasm_store_gc_prepared_at_secs > 0,
        inventory.finalized_at_ns > 0,
        inventory.inventory_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !terminal_facts_are_exact {
        return Err("Fleet Subnet Root final inventory contains invalid terminal authority");
    }
    Ok(())
}
