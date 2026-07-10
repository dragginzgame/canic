//! Module: state_contract
//!
//! Responsibility: declare control-plane stable state metadata keyed by the
//! canonical allocation registry.
//! Does not own: role applicability, CLI rendering, migration execution, or
//! stable-memory access.
//! Boundary: descriptors are static metadata supplied to host-side materialization.

use canic_core::{
    role_contract::{
        AllocationOwner, StateAllocationKey,
        allocation::memory::template::{
            CONTROL_PLANE_SUBNET_STATE_ID, TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID,
            TEMPLATE_CHUNK_SETS_ID, TEMPLATE_MANIFESTS_ID, WASM_STORE_GC_STATE_ID,
        },
    },
    state_contract::{
        MigrationPolicy, StateAllocationDescriptor, StateDomainManifest, StateStorage,
    },
};

#[must_use]
pub fn canic_control_plane_state_descriptors() -> Vec<StateAllocationDescriptor> {
    vec![
        descriptor(
            StateAllocationKey::TemplateManifests,
            "template_manifests",
            TEMPLATE_MANIFESTS_ID,
            "TemplateManifestRecord",
            "TemplateManifestData",
            200,
            "template_manifests_restore_release_index",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkSets,
            "template_chunk_sets",
            TEMPLATE_CHUNK_SETS_ID,
            "TemplateChunkSetRecord",
            "TemplateChunkSetData",
            210,
            "template_chunk_sets_restore_release_metadata",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkRefs,
            "template_chunk_refs",
            TEMPLATE_CHUNK_REFS_ID,
            "TemplateChunkRefRecord",
            "TemplateChunkRefData",
            220,
            "template_chunk_refs_restore_chunk_slots",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkPayloads,
            "template_chunk_payloads",
            TEMPLATE_CHUNK_PAYLOADS_ID,
            "TemplateChunkPayloadRecord",
            "TemplateChunkPayloadData",
            230,
            "template_chunk_payloads_restore_chunk_bytes",
        ),
        descriptor(
            StateAllocationKey::ControlPlaneSubnetState,
            "control_plane_subnet_state",
            CONTROL_PLANE_SUBNET_STATE_ID,
            "SubnetStateRecord",
            "ControlPlaneSubnetStateData",
            240,
            "control_plane_subnet_state_restores_publication_bindings",
        ),
        descriptor(
            StateAllocationKey::WasmStoreGcState,
            "wasm_store_gc_state",
            WASM_STORE_GC_STATE_ID,
            "WasmStoreGcStateRecord",
            "WasmStoreGcStateData",
            240,
            "wasm_store_gc_state_restores_local_gc_mode",
        ),
    ]
}

fn descriptor(
    allocation: StateAllocationKey,
    domain: &str,
    memory_id: u8,
    record: &str,
    snapshot: &str,
    restore_order: u32,
    invariant: &str,
) -> StateAllocationDescriptor {
    StateAllocationDescriptor {
        allocation,
        owner: AllocationOwner::CanicControlPlane,
        state: vec![StateDomainManifest {
            domain: domain.to_string(),
            version: 1,
            storage: StateStorage::StableMemory,
            memory_id: Some(memory_id),
            owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
            record: record.to_string(),
            snapshot: snapshot.to_string(),
            min_supported_version: 1,
            migration_policy: MigrationPolicy::NewDomain,
            restore_order: Some(restore_order),
            post_upgrade_invariant: Some(invariant.to_string()),
            migrations: Vec::new(),
        }],
        reserved_memory: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_declare_template_control_plane_and_gc_allocations() {
        let descriptors = canic_control_plane_state_descriptors();
        let keys = descriptors
            .iter()
            .map(|descriptor| descriptor.allocation)
            .collect::<Vec<_>>();

        for expected in [
            StateAllocationKey::TemplateManifests,
            StateAllocationKey::TemplateChunkSets,
            StateAllocationKey::TemplateChunkRefs,
            StateAllocationKey::TemplateChunkPayloads,
            StateAllocationKey::ControlPlaneSubnetState,
            StateAllocationKey::WasmStoreGcState,
        ] {
            assert!(keys.contains(&expected));
        }
    }
}
