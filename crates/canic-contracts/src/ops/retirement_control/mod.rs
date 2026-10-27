//! Lossless native lifecycle boundaries; no new draining, transfer or recovery policy.
use crate::dto::{fleet_registry as stored, retirement_control as wire};

impl From<stored::FleetSubnetRootDrainingReservationRequest> for wire::RemovalRequest {
    fn from(value: stored::FleetSubnetRootDrainingReservationRequest) -> Self {
        Self {
            asset_recipient: value.asset_recipient.into(),
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            expected_root: value.expected_root.into(),
        }
    }
}

impl From<wire::RemovalRequest> for stored::FleetSubnetRootDrainingReservationRequest {
    fn from(value: wire::RemovalRequest) -> Self {
        Self {
            asset_recipient: value.asset_recipient.into(),
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            expected_root: value.expected_root.into(),
        }
    }
}

impl From<stored::FleetSubnetRootDrainingReservationResponse> for wire::RemovalReservation {
    fn from(value: stored::FleetSubnetRootDrainingReservationResponse) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            prepared_at_ns: value.prepared_at_ns,
            reservation_hash: value.reservation_hash,
        }
    }
}

impl From<wire::RemovalReservation> for stored::FleetSubnetRootDrainingReservationResponse {
    fn from(value: wire::RemovalReservation) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            prepared_at_ns: value.prepared_at_ns,
            reservation_hash: value.reservation_hash,
        }
    }
}

impl From<stored::FleetRetirementRequest> for wire::RetirementRequest {
    fn from(value: stored::FleetRetirementRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            destination: value.destination.into(),
            maximum_ledger_fee: value.maximum_ledger_fee,
        }
    }
}

impl From<wire::RetirementRequest> for stored::FleetRetirementRequest {
    fn from(value: wire::RetirementRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            destination: value.destination.into(),
            maximum_ledger_fee: value.maximum_ledger_fee,
        }
    }
}

impl From<stored::FleetRetirementStatus> for wire::RetirementStatus {
    fn from(value: stored::FleetRetirementStatus) -> Self {
        Self {
            request: value.request.into(),
            prepared_at_ns: value.prepared_at_ns,
            ledger_transfer: value.ledger_transfer.map(Into::into),
            ledger_receipt: value.ledger_receipt.map(Into::into),
        }
    }
}

impl From<wire::RetirementStatus> for stored::FleetRetirementStatus {
    fn from(value: wire::RetirementStatus) -> Self {
        Self {
            request: value.request.into(),
            prepared_at_ns: value.prepared_at_ns,
            ledger_transfer: value.ledger_transfer.map(Into::into),
            ledger_receipt: value.ledger_receipt.map(Into::into),
        }
    }
}

impl From<stored::FleetLedgerTransferIntent> for wire::LedgerTransferIntent {
    fn from(value: stored::FleetLedgerTransferIntent) -> Self {
        Self {
            source: value.source.into(),
            destination: value.destination.into(),
            balance_before: value.balance_before,
            fee: value.fee,
            created_at_time: value.created_at_time,
            memo: value.memo,
        }
    }
}

impl From<wire::LedgerTransferIntent> for stored::FleetLedgerTransferIntent {
    fn from(value: wire::LedgerTransferIntent) -> Self {
        Self {
            source: value.source.into(),
            destination: value.destination.into(),
            balance_before: value.balance_before,
            fee: value.fee,
            created_at_time: value.created_at_time,
            memo: value.memo,
        }
    }
}

impl From<stored::FleetLedgerTransferReceipt> for wire::LedgerTransferReceipt {
    fn from(value: stored::FleetLedgerTransferReceipt) -> Self {
        Self {
            intent: value.intent.into(),
            block_index: value.block_index,
        }
    }
}

impl From<wire::LedgerTransferReceipt> for stored::FleetLedgerTransferReceipt {
    fn from(value: wire::LedgerTransferReceipt) -> Self {
        Self {
            intent: value.intent.into(),
            block_index: value.block_index,
        }
    }
}

impl From<crate::dto::role::RootRemovalRequest> for wire::RootRemovalRequest {
    fn from(value: crate::dto::role::RootRemovalRequest) -> Self {
        Self {
            reservation: value.reservation.into(),
        }
    }
}
impl From<wire::RootRemovalRequest> for crate::dto::role::RootRemovalRequest {
    fn from(value: wire::RootRemovalRequest) -> Self {
        Self {
            reservation: value.reservation.into(),
        }
    }
}

impl From<crate::dto::fleet_subnet_root::FleetSubnetRootDrainingResponse> for wire::Draining {
    fn from(value: crate::dto::fleet_subnet_root::FleetSubnetRootDrainingResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            placement_subnet: value.placement_subnet.into_principal().into(),
            active_registry: value.active_registry.into(),
            reservation_hash: value.reservation_hash,
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            next_allocation_sequence: value.next_allocation_sequence,
            reserved_component_instances: value.reserved_component_instances,
            committed_component_instances: value.committed_component_instances,
            managed_descendants: value.managed_descendants,
            known_created_component_canisters: value.known_created_component_canisters,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            started_at_ns: value.started_at_ns,
        }
    }
}

impl From<wire::Draining> for crate::dto::fleet_subnet_root::FleetSubnetRootDrainingResponse {
    fn from(value: wire::Draining) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            placement_subnet: crate::ids::SubnetId::from_principal(value.placement_subnet.into()),
            active_registry: value.active_registry.into(),
            reservation_hash: value.reservation_hash,
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            next_allocation_sequence: value.next_allocation_sequence,
            reserved_component_instances: value.reserved_component_instances,
            committed_component_instances: value.committed_component_instances,
            managed_descendants: value.managed_descendants,
            known_created_component_canisters: value.known_created_component_canisters,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            started_at_ns: value.started_at_ns,
        }
    }
}

impl From<crate::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse>
    for wire::FinalInventory
{
    fn from(value: crate::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            placement_subnet: value.placement_subnet.into_principal().into(),
            registry: value.registry.into(),
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            next_allocation_sequence: value.next_allocation_sequence,
            removed_component_instances: value.removed_component_instances,
            terminal_component_history_hash: value.terminal_component_history_hash,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            wasm_store: value.wasm_store.into(),
            wasm_store_catalog_hash: value.wasm_store_catalog_hash,
            wasm_store_catalog_entries: value.wasm_store_catalog_entries,
            wasm_store_occupied_bytes: value.wasm_store_occupied_bytes,
            wasm_store_template_count: value.wasm_store_template_count,
            wasm_store_release_count: value.wasm_store_release_count,
            wasm_store_gc_prepared_at_secs: value.wasm_store_gc_prepared_at_secs,
            finalized_at_ns: value.finalized_at_ns,
            inventory_hash: value.inventory_hash,
        }
    }
}

impl From<wire::FinalInventory>
    for crate::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse
{
    fn from(value: wire::FinalInventory) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            placement_subnet: crate::ids::SubnetId::from_principal(value.placement_subnet.into()),
            registry: value.registry.into(),
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            next_allocation_sequence: value.next_allocation_sequence,
            removed_component_instances: value.removed_component_instances,
            terminal_component_history_hash: value.terminal_component_history_hash,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            wasm_store: value.wasm_store.into(),
            wasm_store_catalog_hash: value.wasm_store_catalog_hash,
            wasm_store_catalog_entries: value.wasm_store_catalog_entries,
            wasm_store_occupied_bytes: value.wasm_store_occupied_bytes,
            wasm_store_template_count: value.wasm_store_template_count,
            wasm_store_release_count: value.wasm_store_release_count,
            wasm_store_gc_prepared_at_secs: value.wasm_store_gc_prepared_at_secs,
            finalized_at_ns: value.finalized_at_ns,
            inventory_hash: value.inventory_hash,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse>
    for wire::DrainingPublication
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse) -> Self {
        Self {
            root_draining: value.root_draining.into(),
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}

impl From<wire::DrainingPublication>
    for crate::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse
{
    fn from(value: wire::DrainingPublication) -> Self {
        Self {
            root_draining: value.root_draining.into(),
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse>
    for wire::RemovalPublication
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse) -> Self {
        Self {
            final_inventory: value.final_inventory.into(),
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}

impl From<wire::RemovalPublication>
    for crate::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse
{
    fn from(value: wire::RemovalPublication) -> Self {
        Self {
            final_inventory: value.final_inventory.into(),
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentRequest>
    for wire::ReadinessIntentRequest
{
    fn from(
        value: crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentRequest,
    ) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<wire::ReadinessIntentRequest>
    for crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentRequest
{
    fn from(value: wire::ReadinessIntentRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentResponse>
    for wire::ReadinessIntent
{
    fn from(
        value: crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentResponse,
    ) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            recorded_at_ns: value.recorded_at_ns,
            intent_hash: value.intent_hash,
        }
    }
}

impl From<wire::ReadinessIntent>
    for crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessIntentResponse
{
    fn from(value: wire::ReadinessIntent) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            recorded_at_ns: value.recorded_at_ns,
            intent_hash: value.intent_hash,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessRequest>
    for wire::ReadinessRequest
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessRequest) -> Self {
        Self {
            ledger_receipt: value.ledger_receipt.into(),
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_intent_hash: value.expected_intent_hash,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
        }
    }
}

impl From<wire::ReadinessRequest>
    for crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessRequest
{
    fn from(value: wire::ReadinessRequest) -> Self {
        Self {
            ledger_receipt: value.ledger_receipt.into(),
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_intent_hash: value.expected_intent_hash,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse>
    for wire::Readiness
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            prepared_at_ns: value.prepared_at_ns,
            recorded_at_ns: value.recorded_at_ns,
            readiness_hash: value.readiness_hash,
        }
    }
}

impl From<wire::Readiness>
    for crate::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse
{
    fn from(value: wire::Readiness) -> Self {
        Self {
            request: value.request.into(),
            coordinator: value.coordinator.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            prepared_at_ns: value.prepared_at_ns,
            recorded_at_ns: value.recorded_at_ns,
            readiness_hash: value.readiness_hash,
        }
    }
}

impl From<crate::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus>
    for wire::RemovalStatus
{
    fn from(value: crate::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            reservation: value.reservation.into(),
            draining: value.draining.map(Into::into),
            removal: value.removal.map(Into::into),
            readiness_intent: value.readiness_intent.map(Into::into),
            readiness: value.readiness.map(Into::into),
            execution: value.execution.map(Into::into),
            completion: value.completion.map(Into::into),
        }
    }
}

impl From<wire::RemovalStatus>
    for crate::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus
{
    fn from(value: wire::RemovalStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            reservation: value.reservation.into(),
            draining: value.draining.map(Into::into),
            removal: value.removal.map(Into::into),
            readiness_intent: value.readiness_intent.map(Into::into),
            readiness: value.readiness.map(Into::into),
            execution: value.execution.map(Into::into),
            completion: value.completion.map(Into::into),
        }
    }
}
