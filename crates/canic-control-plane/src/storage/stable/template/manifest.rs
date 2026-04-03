use crate::ids::{
    CanisterRole, TemplateChunkingMode, TemplateManifestState, TemplateReleaseKey, TemplateVersion,
    WasmStoreBinding,
};
use canic_cdk::structures::{
    BTreeMap, DefaultMemoryImpl, memory::VirtualMemory, storable::Storable,
};
use canic_memory::{eager_static, ic_memory, impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const TEMPLATE_MANIFESTS_ID: u8 = 10;

eager_static! {
    static TEMPLATE_MANIFESTS: RefCell<
        BTreeMap<TemplateReleaseKey, TemplateManifestRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateManifestStateStore, TEMPLATE_MANIFESTS_ID)),
    );
}

///
/// TemplateManifestRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateManifestRecord {
    pub role: CanisterRole,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub store_binding: WasmStoreBinding,
    pub chunking_mode: TemplateChunkingMode,
    pub manifest_state: TemplateManifestState,
    pub approved_at: Option<u64>,
    pub created_at: u64,
}

impl TemplateManifestRecord {
    pub const STORABLE_MAX_SIZE: u32 = 512;
}

impl_storable_bounded!(
    TemplateManifestRecord,
    TemplateManifestRecord::STORABLE_MAX_SIZE,
    false
);

///
/// TemplateManifestStoreRecord
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateManifestStoreRecord {
    pub entries: Vec<(TemplateReleaseKey, TemplateManifestRecord)>,
}

///
/// TemplateManifestStateStore
///

pub struct TemplateManifestStateStore;

impl TemplateManifestStateStore {
    // Insert or replace a stored template manifest record.
    pub fn upsert(release: TemplateReleaseKey, record: TemplateManifestRecord) {
        TEMPLATE_MANIFESTS.with_borrow_mut(|map| {
            map.insert(release, record);
        });
    }

    // Export the full manifest snapshot for ops-owned filtering and shaping.
    #[must_use]
    pub fn export() -> TemplateManifestStoreRecord {
        TEMPLATE_MANIFESTS.with_borrow(|map| TemplateManifestStoreRecord {
            entries: map
                .iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect(),
        })
    }

    // Return current manifest-store occupied bytes without cloning the full snapshot.
    #[must_use]
    pub fn occupied_bytes() -> u64 {
        TEMPLATE_MANIFESTS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().to_bytes().len() + entry.value().to_bytes().len()) as u64)
                .sum()
        })
    }

    // Clear the manifest store.
    pub fn clear() {
        TEMPLATE_MANIFESTS.with_borrow_mut(BTreeMap::clear);
    }

    // Clear the manifest store for isolated unit tests.
    #[cfg(test)]
    pub fn clear_for_test() {
        Self::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TemplateId;

    fn manifest() -> TemplateManifestRecord {
        TemplateManifestRecord {
            role: CanisterRole::new("app"),
            version: TemplateVersion::new("0.18.0"),
            payload_hash: vec![7; 32],
            payload_size_bytes: 1024,
            store_binding: WasmStoreBinding::new("primary"),
            chunking_mode: TemplateChunkingMode::Inline,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(42),
            created_at: 41,
        }
    }

    fn release() -> TemplateReleaseKey {
        TemplateReleaseKey::new(
            TemplateId::new("embedded:app"),
            TemplateVersion::new("0.18.0"),
        )
    }

    #[test]
    fn manifest_store_round_trip() {
        TemplateManifestStateStore::clear_for_test();
        let record = manifest();

        TemplateManifestStateStore::upsert(release(), record.clone());

        let exported = TemplateManifestStateStore::export();
        assert_eq!(exported.entries, vec![(release(), record)]);
    }

    #[test]
    fn manifest_store_export_is_sorted_by_key_order() {
        TemplateManifestStateStore::clear_for_test();

        TemplateManifestStateStore::upsert(
            TemplateReleaseKey::new(
                TemplateId::new("embedded:z"),
                TemplateVersion::new("0.18.0"),
            ),
            manifest(),
        );
        TemplateManifestStateStore::upsert(release(), manifest());

        let exported = TemplateManifestStateStore::export();
        assert_eq!(exported.entries.len(), 2);
        assert_eq!(
            exported.entries[0].0.template_id,
            TemplateId::new("embedded:app")
        );
    }
}
