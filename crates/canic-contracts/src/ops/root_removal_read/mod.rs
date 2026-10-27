//! Lossless Root-removal transport projection; no state mutation or paid effects.
use crate::dto::{fleet_subnet_root as stored, root_removal_read as native};

impl From<crate::dto::root::RootRemovalOperationStatus> for native::Status {
    fn from(value: crate::dto::root::RootRemovalOperationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            draining: value.draining.into(),
            final_inventory: value.final_inventory.map(Into::into),
            removal: value.removal.map(Into::into),
            store_reclamation: value.store_reclamation.map(Into::into),
            store_binding_finalization: value.store_binding_finalization.map(Into::into),
            store_deletion: value.store_deletion.map(Into::into),
            deletion_readiness_intent: value.deletion_readiness_intent.map(Into::into),
            deletion_readiness: value.deletion_readiness.map(Into::into),
            deletion_preparation: value.deletion_preparation.map(Into::into),
        }
    }
}

impl From<native::Status> for crate::dto::root::RootRemovalOperationStatus {
    fn from(value: native::Status) -> Self {
        Self {
            operation_id: value.operation_id,
            draining: value.draining.into(),
            final_inventory: value.final_inventory.map(Into::into),
            removal: value.removal.map(Into::into),
            store_reclamation: value.store_reclamation.map(Into::into),
            store_binding_finalization: value.store_binding_finalization.map(Into::into),
            store_deletion: value.store_deletion.map(Into::into),
            deletion_readiness_intent: value.deletion_readiness_intent.map(Into::into),
            deletion_readiness: value.deletion_readiness.map(Into::into),
            deletion_preparation: value.deletion_preparation.map(Into::into),
        }
    }
}

impl From<stored::FleetSubnetRootStoreReclamationResponse> for native::StoreReclamation {
    fn from(value: stored::FleetSubnetRootStoreReclamationResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            final_inventory_hash: value.final_inventory_hash,
            reclaimed_store_bytes: value.reclaimed_store_bytes,
            reclaimed_catalog_entries: value.reclaimed_catalog_entries,
            reclaimed_template_count: value.reclaimed_template_count,
            reclaimed_release_count: value.reclaimed_release_count,
            gc_prepared_at_secs: value.gc_prepared_at_secs,
            gc_started_at_secs: value.gc_started_at_secs,
            gc_completed_at_secs: value.gc_completed_at_secs,
            gc_runs_completed: value.gc_runs_completed,
            completed_at_ns: value.completed_at_ns,
            reclamation_hash: value.reclamation_hash,
        }
    }
}

impl From<native::StoreReclamation> for stored::FleetSubnetRootStoreReclamationResponse {
    fn from(value: native::StoreReclamation) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            final_inventory_hash: value.final_inventory_hash,
            reclaimed_store_bytes: value.reclaimed_store_bytes,
            reclaimed_catalog_entries: value.reclaimed_catalog_entries,
            reclaimed_template_count: value.reclaimed_template_count,
            reclaimed_release_count: value.reclaimed_release_count,
            gc_prepared_at_secs: value.gc_prepared_at_secs,
            gc_started_at_secs: value.gc_started_at_secs,
            gc_completed_at_secs: value.gc_completed_at_secs,
            gc_runs_completed: value.gc_runs_completed,
            completed_at_ns: value.completed_at_ns,
            reclamation_hash: value.reclamation_hash,
        }
    }
}

impl From<stored::FleetSubnetRootStoreBindingFinalizationResponse> for native::StoreFinalization {
    fn from(value: stored::FleetSubnetRootStoreBindingFinalizationResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            final_inventory_hash: value.final_inventory_hash,
            reclamation_hash: value.reclamation_hash,
            source_generation: value.source_generation,
            finalized_generation: value.finalized_generation,
            finalized_at_secs: value.finalized_at_secs,
            completed_at_ns: value.completed_at_ns,
            finalization_hash: value.finalization_hash,
        }
    }
}

impl From<native::StoreFinalization> for stored::FleetSubnetRootStoreBindingFinalizationResponse {
    fn from(value: native::StoreFinalization) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            final_inventory_hash: value.final_inventory_hash,
            reclamation_hash: value.reclamation_hash,
            source_generation: value.source_generation,
            finalized_generation: value.finalized_generation,
            finalized_at_secs: value.finalized_at_secs,
            completed_at_ns: value.completed_at_ns,
            finalization_hash: value.finalization_hash,
        }
    }
}

impl From<stored::FleetSubnetRootStoreDeletionResponse> for native::StoreDeletion {
    fn from(value: stored::FleetSubnetRootStoreDeletionResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            binding_finalization_hash: value.binding_finalization_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
            prepared_at_ns: value.prepared_at_ns,
            observed_absent_at_ns: value.observed_absent_at_ns,
            completed_at_ns: value.completed_at_ns,
            deletion_hash: value.deletion_hash,
        }
    }
}

impl From<native::StoreDeletion> for stored::FleetSubnetRootStoreDeletionResponse {
    fn from(value: native::StoreDeletion) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            binding_finalization_hash: value.binding_finalization_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
            prepared_at_ns: value.prepared_at_ns,
            observed_absent_at_ns: value.observed_absent_at_ns,
            completed_at_ns: value.completed_at_ns,
            deletion_hash: value.deletion_hash,
        }
    }
}

impl From<stored::FleetSubnetRootDeletionPreparationResponse> for native::Preparation {
    fn from(value: stored::FleetSubnetRootDeletionPreparationResponse) -> Self {
        Self {
            ledger_receipt: value.ledger_receipt.into(),
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
            coordinator_intent_hash: value.coordinator_intent_hash,
            coordinator_readiness_hash: value.coordinator_readiness_hash,
            prepared_at_ns: value.prepared_at_ns,
            completed_at_ns: value.completed_at_ns,
        }
    }
}

impl From<native::Preparation> for stored::FleetSubnetRootDeletionPreparationResponse {
    fn from(value: native::Preparation) -> Self {
        Self {
            ledger_receipt: value.ledger_receipt.into(),
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            final_inventory_hash: value.final_inventory_hash,
            store_deletion_hash: value.store_deletion_hash,
            observed_cycles_before_reclamation: value.observed_cycles_before_reclamation,
            retained_cycles_target: value.retained_cycles_target,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            cycles_reclaimed_at_ns: value.cycles_reclaimed_at_ns,
            coordinator_intent_hash: value.coordinator_intent_hash,
            coordinator_readiness_hash: value.coordinator_readiness_hash,
            prepared_at_ns: value.prepared_at_ns,
            completed_at_ns: value.completed_at_ns,
        }
    }
}
