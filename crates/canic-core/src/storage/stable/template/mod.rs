use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::{
        CanisterRole, TemplateChunkKey, TemplateChunkingMode, TemplateManifestState,
        TemplateReleaseKey, TemplateVersion, WasmStoreBinding,
    },
    storage::stable::memory::topology::{
        TEMPLATE_CHUNK_SETS_ID, TEMPLATE_CHUNKS_ID, TEMPLATE_MANIFESTS_ID,
    },
};
use crate::{
    memory::{impl_storable_bounded, impl_storable_unbounded},
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static TEMPLATE_MANIFESTS: RefCell<
        BTreeMap<TemplateReleaseKey, TemplateManifestRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateManifestStateStore, TEMPLATE_MANIFESTS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNK_SETS: RefCell<
        BTreeMap<TemplateReleaseKey, TemplateChunkSetRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateChunkSetStateStore, TEMPLATE_CHUNK_SETS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNKS: RefCell<
        BTreeMap<TemplateChunkKey, TemplateChunkRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateChunkStore, TEMPLATE_CHUNKS_ID)),
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
/// TemplateChunkSetRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateChunkSetRecord {
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub chunk_count: u32,
    pub chunk_hashes: Vec<Vec<u8>>,
    pub created_at: u64,
}

impl_storable_unbounded!(TemplateChunkSetRecord);

///
/// TemplateChunkRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateChunkRecord {
    pub bytes: Vec<u8>,
}

impl_storable_unbounded!(TemplateChunkRecord);

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
    pub(crate) fn upsert(release: TemplateReleaseKey, record: TemplateManifestRecord) {
        TEMPLATE_MANIFESTS.with_borrow_mut(|map| {
            map.insert(release, record);
        });
    }

    // Export the full manifest snapshot for ops-owned filtering and shaping.
    #[must_use]
    pub(crate) fn export() -> TemplateManifestStoreRecord {
        TEMPLATE_MANIFESTS.with_borrow(|map| TemplateManifestStoreRecord {
            entries: map
                .iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect(),
        })
    }

    // Clear the manifest store.
    pub(crate) fn clear() {
        TEMPLATE_MANIFESTS.with_borrow_mut(BTreeMap::clear);
    }

    // Clear the manifest store for isolated unit tests.
    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        Self::clear();
    }
}

///
/// TemplateChunkSetStateStore
///

pub struct TemplateChunkSetStateStore;

impl TemplateChunkSetStateStore {
    // Insert or replace one template chunk-set metadata record.
    pub(crate) fn upsert(release: TemplateReleaseKey, record: TemplateChunkSetRecord) {
        TEMPLATE_CHUNK_SETS.with_borrow_mut(|map| {
            map.insert(release, record);
        });
    }

    // Fetch one template chunk-set metadata record, if present.
    #[must_use]
    pub(crate) fn get(release: &TemplateReleaseKey) -> Option<TemplateChunkSetRecord> {
        TEMPLATE_CHUNK_SETS.with_borrow(|map| map.get(release))
    }

    // Export the full chunk-set metadata snapshot for ops-owned accounting.
    #[must_use]
    pub(crate) fn export() -> Vec<(TemplateReleaseKey, TemplateChunkSetRecord)> {
        TEMPLATE_CHUNK_SETS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    // Clear the chunk-set metadata store.
    pub(crate) fn clear() {
        TEMPLATE_CHUNK_SETS.with_borrow_mut(BTreeMap::clear);
    }

    // Clear the chunk-set metadata store for isolated unit tests.
    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        Self::clear();
    }
}

///
/// TemplateChunkStore
///

pub struct TemplateChunkStore;

impl TemplateChunkStore {
    // Insert or replace one template chunk.
    pub(crate) fn upsert(chunk_key: TemplateChunkKey, record: TemplateChunkRecord) {
        TEMPLATE_CHUNKS.with_borrow_mut(|map| {
            map.insert(chunk_key, record);
        });
    }

    // Fetch one template chunk, if present.
    #[must_use]
    pub(crate) fn get(chunk_key: &TemplateChunkKey) -> Option<TemplateChunkRecord> {
        TEMPLATE_CHUNKS.with_borrow(|map| map.get(chunk_key))
    }

    // Export the full chunk snapshot for ops-owned accounting.
    #[must_use]
    pub(crate) fn export() -> Vec<(TemplateChunkKey, TemplateChunkRecord)> {
        TEMPLATE_CHUNKS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    // Clear the chunk store.
    pub(crate) fn clear() {
        TEMPLATE_CHUNKS.with_borrow_mut(BTreeMap::clear);
    }

    // Clear the chunk store for isolated unit tests.
    #[cfg(test)]
    pub(crate) fn clear_for_test() {
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
    fn upsert_and_get_manifest_round_trip() {
        TemplateManifestStateStore::clear_for_test();

        let release = TemplateReleaseKey::new(
            TemplateId::new("embedded:app"),
            TemplateVersion::new("0.18.0"),
        );
        let record = manifest();

        TemplateManifestStateStore::upsert(release.clone(), record.clone());

        let exported = TemplateManifestStateStore::export();
        assert_eq!(exported.entries, vec![(release, record)]);
    }

    #[test]
    fn export_returns_all_manifest_entries() {
        TemplateManifestStateStore::clear_for_test();

        TemplateManifestStateStore::upsert(
            TemplateReleaseKey::new(TemplateId::new("one"), TemplateVersion::new("0.18.0")),
            manifest(),
        );
        TemplateManifestStateStore::upsert(
            TemplateReleaseKey::new(TemplateId::new("two"), TemplateVersion::new("0.18.2")),
            TemplateManifestRecord {
                role: CanisterRole::new("scale"),
                store_binding: WasmStoreBinding::new("secondary"),
                ..manifest()
            },
        );

        let exported = TemplateManifestStateStore::export();
        assert_eq!(exported.entries.len(), 2);
    }

    #[test]
    fn upsert_and_get_chunk_set_round_trip() {
        TemplateChunkSetStateStore::clear_for_test();

        let record = TemplateChunkSetRecord {
            payload_hash: vec![9; 32],
            payload_size_bytes: 2048,
            chunk_count: 2,
            chunk_hashes: vec![vec![1; 32], vec![2; 32]],
            created_at: 12,
        };

        TemplateChunkSetStateStore::upsert(release(), record.clone());

        assert_eq!(TemplateChunkSetStateStore::get(&release()), Some(record));
    }

    #[test]
    fn upsert_and_get_chunk_round_trip() {
        TemplateChunkStore::clear_for_test();

        let chunk_key = TemplateChunkKey::new(release(), 1);
        let record = TemplateChunkRecord {
            bytes: vec![1, 2, 3, 4],
        };

        TemplateChunkStore::upsert(chunk_key.clone(), record.clone());

        assert_eq!(TemplateChunkStore::get(&chunk_key), Some(record));
    }
}
