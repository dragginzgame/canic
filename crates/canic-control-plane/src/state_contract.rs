//! Module: state_contract
//!
//! Responsibility: declare control-plane stable state metadata for host-side
//! state manifest and audit reports.
//! Does not own: CLI rendering, migration execution, stable-memory reads, or
//! stable-memory writes.
//! Boundary: declarations are static Rust metadata derived from the storage
//! modules that own the records and memory IDs.

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
use canic_core::state_contract::{
    MigrationPolicy, StateDomainManifest, StateRoleManifest, StateStorage,
};
use canic_core::state_contract::{STATE_MANIFEST_SCHEMA_VERSION, StateManifest};

#[cfg(feature = "wasm-store-canister")]
use crate::storage::stable::template::gc::WASM_STORE_GC_STATE_ID;
#[cfg(feature = "root-control-plane")]
use crate::storage::stable::{
    state::subnet::SUBNET_STATE_ID,
    template::{
        chunked::{TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID, TEMPLATE_CHUNK_SETS_ID},
        manifest::TEMPLATE_MANIFESTS_ID,
    },
};

#[cfg(feature = "root-control-plane")]
const ROOT_ROLE: &str = "root";
#[cfg(feature = "wasm-store-canister")]
const WASM_STORE_ROLE: &str = "wasm_store";
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
const OWNER: &str = "canic-control-plane";

#[must_use]
pub fn canic_control_plane_state_manifest() -> StateManifest {
    StateManifest {
        schema_version: STATE_MANIFEST_SCHEMA_VERSION,
        roles: declared_roles(),
    }
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
fn declared_roles() -> Vec<StateRoleManifest> {
    #[cfg(feature = "root-control-plane")]
    let root_roles = [root_role_manifest()];
    #[cfg(not(feature = "root-control-plane"))]
    let root_roles = [];

    #[cfg(feature = "wasm-store-canister")]
    let wasm_store_roles = [wasm_store_role_manifest()];
    #[cfg(not(feature = "wasm-store-canister"))]
    let wasm_store_roles = [];

    root_roles.into_iter().chain(wasm_store_roles).collect()
}

#[cfg(not(any(feature = "root-control-plane", feature = "wasm-store-canister")))]
fn declared_roles() -> Vec<canic_core::state_contract::StateRoleManifest> {
    Vec::new()
}

#[cfg(feature = "root-control-plane")]
fn root_role_manifest() -> StateRoleManifest {
    StateRoleManifest {
        canister_role: ROOT_ROLE.to_string(),
        state: vec![
            state_domain(
                "template_manifests",
                TEMPLATE_MANIFESTS_ID,
                "TemplateManifestRecord",
                "TemplateManifestData",
                200,
                "template_manifests_restore_release_index",
            ),
            state_domain(
                "template_chunk_sets",
                TEMPLATE_CHUNK_SETS_ID,
                "TemplateChunkSetRecord",
                "TemplateChunkSetData",
                210,
                "template_chunk_sets_restore_release_metadata",
            ),
            state_domain(
                "template_chunk_refs",
                TEMPLATE_CHUNK_REFS_ID,
                "TemplateChunkRefRecord",
                "TemplateChunkRefData",
                220,
                "template_chunk_refs_restore_chunk_slots",
            ),
            state_domain(
                "template_chunk_payloads",
                TEMPLATE_CHUNK_PAYLOADS_ID,
                "TemplateChunkPayloadRecord",
                "TemplateChunkPayloadData",
                230,
                "template_chunk_payloads_restore_chunk_bytes",
            ),
            state_domain(
                "control_plane_subnet_state",
                SUBNET_STATE_ID,
                "SubnetStateRecord",
                "ControlPlaneSubnetStateData",
                240,
                "control_plane_subnet_state_restores_publication_bindings",
            ),
        ],
        removed_state: Vec::new(),
        reserved_memory: Vec::new(),
    }
}

#[cfg(feature = "wasm-store-canister")]
fn wasm_store_role_manifest() -> StateRoleManifest {
    StateRoleManifest {
        canister_role: WASM_STORE_ROLE.to_string(),
        state: vec![state_domain(
            "wasm_store_gc_state",
            WASM_STORE_GC_STATE_ID,
            "WasmStoreGcStateRecord",
            "WasmStoreGcStateData",
            10,
            "wasm_store_gc_state_restores_local_gc_mode",
        )],
        removed_state: Vec::new(),
        reserved_memory: Vec::new(),
    }
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
fn state_domain(
    domain: &str,
    memory_id: u8,
    record: &str,
    snapshot: &str,
    restore_order: u32,
    invariant: &str,
) -> StateDomainManifest {
    StateDomainManifest {
        domain: domain.to_string(),
        version: 1,
        storage: StateStorage::StableMemory,
        memory_id: Some(memory_id),
        owner: OWNER.to_string(),
        record: record.to_string(),
        snapshot: snapshot.to_string(),
        min_supported_version: 1,
        migration_policy: MigrationPolicy::NewDomain,
        restore_order: Some(restore_order),
        post_upgrade_invariant: Some(invariant.to_string()),
        migrations: Vec::new(),
    }
}

#[cfg(all(
    test,
    any(feature = "root-control-plane", feature = "wasm-store-canister")
))]
mod tests {
    use super::*;

    #[cfg(feature = "root-control-plane")]
    #[test]
    fn control_plane_manifest_declares_owned_memory_ids() {
        let manifest = canic_control_plane_state_manifest();
        let root = manifest
            .roles
            .iter()
            .find(|role| role.canister_role == ROOT_ROLE)
            .expect("root role");
        let ids = root
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&TEMPLATE_MANIFESTS_ID));
        assert!(ids.contains(&TEMPLATE_CHUNK_SETS_ID));
        assert!(ids.contains(&TEMPLATE_CHUNK_REFS_ID));
        assert!(ids.contains(&TEMPLATE_CHUNK_PAYLOADS_ID));
        assert!(ids.contains(&SUBNET_STATE_ID));
    }

    #[cfg(feature = "wasm-store-canister")]
    #[test]
    fn wasm_store_manifest_declares_gc_state() {
        let manifest = canic_control_plane_state_manifest();
        let wasm_store = manifest
            .roles
            .iter()
            .find(|role| role.canister_role == WASM_STORE_ROLE)
            .expect("wasm_store role");

        assert!(wasm_store.state.iter().any(|domain| {
            domain.domain == "wasm_store_gc_state"
                && domain.memory_id == Some(WASM_STORE_GC_STATE_ID)
        }));
    }
}
