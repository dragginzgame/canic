use crate::ids::{
    CanisterRole, TemplateChunkingMode, TemplateManifestState, TemplateReleaseKey, TemplateVersion,
    WasmStoreBinding,
};
use canic_core::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use canic_core::cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory, storable::Storable};
use canic_core::eager_static;
use canic_core::{
    impl_storable_bounded, role_contract::allocation::memory::template::TEMPLATE_MANIFESTS_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static TEMPLATE_MANIFESTS: RefCell<
        StableBtreeMap<TemplateReleaseKey, TemplateManifestRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(canic_core::ic_memory_key!(authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, key = "canic.control_plane.template_manifest.v1", ty = TemplateManifestStateStore, id = TEMPLATE_MANIFESTS_ID)),
    );
}

eager_static! {
    static TEMPLATE_MANIFESTS_OCCUPIED_BYTES: RefCell<Option<u64>> = RefCell::new(None);
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
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateManifestRecord";
    pub const STORABLE_MAX_SIZE: u32 = 512;
}

impl_storable_bounded!(
    TemplateManifestRecord,
    TemplateManifestRecord::STORABLE_MAX_SIZE,
    false
);

///
/// TemplateManifestEntryRecord
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateManifestEntryRecord {
    pub release: TemplateReleaseKey,
    pub record: TemplateManifestRecord,
}

///
/// TemplateManifestsData
///
/// Canonical template-manifest allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateManifestsData {
    pub entries: Vec<TemplateManifestEntryRecord>,
}

impl TemplateManifestsData {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateManifestsData";
}

///
/// TemplateManifestStateStore
///

pub struct TemplateManifestStateStore;

impl TemplateManifestStateStore {
    // Insert or replace a stored template manifest record.
    pub fn upsert(release: TemplateReleaseKey, record: TemplateManifestRecord) {
        TEMPLATE_MANIFESTS.with_borrow_mut(|map| {
            let previous = map.insert(release.clone(), record.clone());
            TEMPLATE_MANIFESTS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
                if let Some(current) = occupied.as_mut() {
                    let previous_bytes = previous
                        .as_ref()
                        .map_or(0, |previous| manifest_entry_size(&release, previous));
                    let next_bytes = manifest_entry_size(&release, &record);
                    *current = current
                        .saturating_sub(previous_bytes)
                        .saturating_add(next_bytes);
                }
            });
        });
    }

    // Remove one stored template manifest record.
    pub fn remove(release: &TemplateReleaseKey) -> Option<TemplateManifestRecord> {
        TEMPLATE_MANIFESTS.with_borrow_mut(|map| {
            let removed = map.remove(release);
            TEMPLATE_MANIFESTS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
                if let (Some(current), Some(record)) = (occupied.as_mut(), removed.as_ref()) {
                    *current = current.saturating_sub(manifest_entry_size(release, record));
                }
            });
            removed
        })
    }

    // Export the full manifest snapshot for ops-owned filtering and shaping.
    #[must_use]
    pub fn export() -> TemplateManifestsData {
        TEMPLATE_MANIFESTS.with_borrow(|map| TemplateManifestsData {
            entries: map
                .iter()
                .map(|entry| TemplateManifestEntryRecord {
                    release: entry.key().clone(),
                    record: entry.value(),
                })
                .collect(),
        })
    }

    #[cfg(test)]
    pub fn import(data: TemplateManifestsData) {
        Self::clear();
        for entry in data.entries {
            Self::upsert(entry.release, entry.record);
        }
    }

    // Return current manifest-store occupied bytes without cloning the full snapshot.
    #[must_use]
    pub fn occupied_bytes() -> u64 {
        if let Some(bytes) = TEMPLATE_MANIFESTS_OCCUPIED_BYTES.with_borrow(|occupied| *occupied) {
            return bytes;
        }

        let bytes = TEMPLATE_MANIFESTS.with_borrow(|map| {
            map.iter()
                .map(|entry| manifest_entry_size(entry.key(), &entry.value()))
                .sum()
        });
        TEMPLATE_MANIFESTS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(bytes);
        });

        bytes
    }

    // Clear the manifest store.
    pub fn clear() {
        TEMPLATE_MANIFESTS.with_borrow_mut(StableBtreeMap::clear_new);
        TEMPLATE_MANIFESTS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(0);
        });
    }

    // Clear the manifest store for isolated unit tests.
    #[cfg(test)]
    pub fn clear_for_test() {
        Self::clear();
    }
}

fn manifest_entry_size(release: &TemplateReleaseKey, record: &TemplateManifestRecord) -> u64 {
    (release.to_bytes().len() + record.to_bytes().len()) as u64
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
        assert_eq!(
            exported.entries,
            vec![TemplateManifestEntryRecord {
                release: release(),
                record,
            }]
        );
    }

    #[test]
    fn manifest_store_remove_updates_occupied_bytes() {
        TemplateManifestStateStore::clear_for_test();
        let record = manifest();
        TemplateManifestStateStore::upsert(release(), record.clone());

        assert!(TemplateManifestStateStore::occupied_bytes() > 0);
        assert_eq!(TemplateManifestStateStore::remove(&release()), Some(record));
        assert!(TemplateManifestStateStore::export().entries.is_empty());
        assert_eq!(TemplateManifestStateStore::occupied_bytes(), 0);
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
            exported.entries[0].release.template_id,
            TemplateId::new("embedded:app")
        );
    }

    #[test]
    fn manifest_store_round_trips_through_canonical_data_snapshot() {
        TemplateManifestStateStore::clear_for_test();
        TemplateManifestStateStore::upsert(release(), manifest());

        let data = TemplateManifestStateStore::export();
        TemplateManifestStateStore::clear_for_test();
        TemplateManifestStateStore::import(data.clone());

        assert_eq!(TemplateManifestStateStore::export(), data);
        TemplateManifestStateStore::clear_for_test();
    }
}
