//! Stable keys shared by store opens and the complete component request factory.
use canic_core::__reexports::ic_memory;

macro_rules! keys {
    ($( $(#[$attr:meta])* $name:ident = $key:literal; )*) => {
        $( $(#[$attr])* pub const $name: &str = $key; )*
        pub(super) fn requests() -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
            Ok(vec![$( $(#[$attr])* ic_memory::MemoryRequest::new(super::MEMORY_AUTHORITY, $name)?, )*])
        }
    };
}

keys! {
    #[cfg(feature = "wasm-store-canister")]
    FIXTURE_STORE_V1 = "canic.control_plane.fixture_store.v1";
    #[cfg(feature = "fleet-coordinator-canister")]
    FLEET_ADMISSION_V1 = "canic.control_plane.fleet_admission.v1";
    #[cfg(feature = "fleet-coordinator-canister")]
    FLEET_COORDINATOR_FUNDING_V1 = "canic.control_plane.fleet_coordinator.funding.v1";
    #[cfg(feature = "fleet-coordinator-canister")]
    FLEET_COORDINATOR_REGISTRY_V1 = "canic.control_plane.fleet_coordinator.registry.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_ADMISSION_V1 = "canic.control_plane.root.admission.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_CANISTER_INVENTORY_ASSETS_V1 = "canic.control_plane.root.canister_inventory.assets.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_V1 = "canic.control_plane.root.canister_pool.handoff_receipts.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_CANISTER_POOL_STATE_V1 = "canic.control_plane.root.canister_pool.state.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_ALLOCATIONS_V1 = "canic.control_plane.root.component.allocations.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_DRAINING_V1 = "canic.control_plane.root.component.draining.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_PRINCIPAL_INDEX_V1 = "canic.control_plane.root.component.principal_index.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_PROVISIONING_OPERATIONS_V1 = "canic.control_plane.root.component_provisioning.operations.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_PROVISIONING_PLACEMENTS_V1 = "canic.control_plane.root.component_provisioning.placements.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_PROVISIONING_STATE_V1 = "canic.control_plane.root.component_provisioning.state.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_REGISTRY_ENTRIES_V1 = "canic.control_plane.root.component.registry_entries.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_REGISTRY_STATE_V1 = "canic.control_plane.root.component.registry_state.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_V1 = "canic.control_plane.root.component.subtree_removal_history.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_FLEET_REGISTRY_MIRROR_V1 = "canic.control_plane.root.fleet_registry_mirror.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_FUNDING_V1 = "canic.control_plane.root.funding.v1";
    #[cfg(feature = "root-control-plane")]
    ROOT_WASM_STORE_STATE_V1 = "canic.control_plane.root.wasm_store.state.v1";
    #[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
    TEMPLATE_CHUNK_PAYLOADS_V1 = "canic.control_plane.template.chunk_payloads.v1";
    #[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
    TEMPLATE_CHUNK_REFS_V1 = "canic.control_plane.template.chunk_refs.v1";
    #[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
    TEMPLATE_CHUNK_SETS_V1 = "canic.control_plane.template.chunk_sets.v1";
    #[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
    TEMPLATE_MANIFESTS_V1 = "canic.control_plane.template.manifests.v1";
    #[cfg(feature = "wasm-store-canister")]
    WASM_STORE_GC_STATE_V1 = "canic.control_plane.wasm_store.gc_state.v1";
}
