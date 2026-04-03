use crate::ids::{TemplateChunkKey, TemplateReleaseKey};
use canic_cdk::structures::{
    BTreeMap, DefaultMemoryImpl, memory::VirtualMemory, storable::Storable,
};
use canic_memory::{eager_static, ic_memory, impl_storable_unbounded};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap as StdBTreeMap};

const TEMPLATE_CHUNK_SETS_ID: u8 = 11;
const TEMPLATE_CHUNKS_ID: u8 = 12;

eager_static! {
    static TEMPLATE_CHUNK_SETS: RefCell<
        BTreeMap<TemplateReleaseKey, TemplateChunkSetRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateChunkSetStateStore, TEMPLATE_CHUNK_SETS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES: RefCell<Option<u64>> = RefCell::new(None);
}

eager_static! {
    static TEMPLATE_CHUNKS: RefCell<
        BTreeMap<TemplateChunkKey, TemplateChunkRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateChunkStore, TEMPLATE_CHUNKS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNKS_OCCUPIED_BYTES: RefCell<Option<u64>> = RefCell::new(None);
}

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
/// TemplateChunkSetStateStore
///

pub struct TemplateChunkSetStateStore;

impl TemplateChunkSetStateStore {
    // Insert or replace one template chunk-set metadata record.
    pub fn upsert(release: TemplateReleaseKey, record: TemplateChunkSetRecord) {
        TEMPLATE_CHUNK_SETS.with_borrow_mut(|map| {
            let previous = map.insert(release.clone(), record.clone());
            TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
                if let Some(current) = occupied.as_mut() {
                    let previous_bytes = previous
                        .as_ref()
                        .map_or(0, |previous| chunk_set_entry_size(&release, previous));
                    let next_bytes = chunk_set_entry_size(&release, &record);
                    *current = current
                        .saturating_sub(previous_bytes)
                        .saturating_add(next_bytes);
                }
            });
        });
    }

    // Fetch one template chunk-set metadata record, if present.
    #[must_use]
    pub fn get(release: &TemplateReleaseKey) -> Option<TemplateChunkSetRecord> {
        TEMPLATE_CHUNK_SETS.with_borrow(|map| map.get(release))
    }

    // Export the full chunk-set metadata snapshot for ops-owned accounting.
    #[must_use]
    pub fn export() -> Vec<(TemplateReleaseKey, TemplateChunkSetRecord)> {
        TEMPLATE_CHUNK_SETS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    // Return current chunk-set occupied bytes without cloning the full snapshot.
    #[must_use]
    pub fn occupied_bytes() -> u64 {
        if let Some(bytes) = TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES.with_borrow(|occupied| *occupied) {
            return bytes;
        }

        let bytes = TEMPLATE_CHUNK_SETS.with_borrow(|map| {
            map.iter()
                .map(|entry| chunk_set_entry_size(entry.key(), &entry.value()))
                .sum()
        });
        TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(bytes);
        });

        bytes
    }

    // Clear the chunk-set metadata store.
    pub fn clear() {
        TEMPLATE_CHUNK_SETS.with_borrow_mut(BTreeMap::clear);
        TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(0);
        });
    }

    // Clear the chunk-set metadata store for isolated unit tests.
    #[cfg(test)]
    pub fn clear_for_test() {
        Self::clear();
    }
}

///
/// TemplateChunkStore
///

pub struct TemplateChunkStore;

impl TemplateChunkStore {
    // Insert or replace one template chunk.
    pub fn upsert(chunk_key: TemplateChunkKey, record: TemplateChunkRecord) {
        TEMPLATE_CHUNKS.with_borrow_mut(|map| {
            let previous = map.insert(chunk_key.clone(), record.clone());
            TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
                if let Some(current) = occupied.as_mut() {
                    let previous_bytes = previous
                        .as_ref()
                        .map_or(0, |previous| chunk_entry_size(&chunk_key, previous));
                    let next_bytes = chunk_entry_size(&chunk_key, &record);
                    *current = current
                        .saturating_sub(previous_bytes)
                        .saturating_add(next_bytes);
                }
            });
        });
    }

    // Fetch one template chunk, if present.
    #[must_use]
    pub fn get(chunk_key: &TemplateChunkKey) -> Option<TemplateChunkRecord> {
        TEMPLATE_CHUNKS.with_borrow(|map| map.get(chunk_key))
    }

    // Export the full chunk snapshot for ops-owned accounting.
    #[must_use]
    pub fn export() -> Vec<(TemplateChunkKey, TemplateChunkRecord)> {
        TEMPLATE_CHUNKS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    // Return current chunk occupied bytes without cloning the full chunk snapshot.
    #[must_use]
    pub fn occupied_bytes() -> u64 {
        if let Some(bytes) = TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow(|occupied| *occupied) {
            return bytes;
        }

        let bytes = TEMPLATE_CHUNKS.with_borrow(|map| {
            map.iter()
                .map(|entry| chunk_entry_size(entry.key(), &entry.value()))
                .sum()
        });
        TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(bytes);
        });

        bytes
    }

    // Count staged chunks by release without cloning chunk payload bytes.
    #[must_use]
    pub fn count_by_release() -> StdBTreeMap<TemplateReleaseKey, u32> {
        TEMPLATE_CHUNKS.with_borrow(|map| {
            let mut counts: StdBTreeMap<TemplateReleaseKey, u32> = StdBTreeMap::new();

            for entry in map.iter() {
                let release = entry.key().release.clone();
                let count = counts.entry(release).or_insert(0);
                *count = u32::saturating_add(*count, 1);
            }

            counts
        })
    }

    // Clear the chunk store.
    pub fn clear() {
        TEMPLATE_CHUNKS.with_borrow_mut(BTreeMap::clear);
        TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            *occupied = Some(0);
        });
    }

    // Clear the chunk store for isolated unit tests.
    #[cfg(test)]
    pub fn clear_for_test() {
        Self::clear();
    }
}

fn chunk_set_entry_size(release: &TemplateReleaseKey, record: &TemplateChunkSetRecord) -> u64 {
    (release.to_bytes().len() + record.to_bytes().len()) as u64
}

fn chunk_entry_size(chunk_key: &TemplateChunkKey, record: &TemplateChunkRecord) -> u64 {
    (chunk_key.to_bytes().len() + record.to_bytes().len()) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{TemplateId, TemplateVersion};

    fn release() -> TemplateReleaseKey {
        TemplateReleaseKey::new(
            TemplateId::new("embedded:app"),
            TemplateVersion::new("0.18.0"),
        )
    }

    #[test]
    fn chunk_set_store_round_trip() {
        TemplateChunkSetStateStore::clear_for_test();
        let record = TemplateChunkSetRecord {
            payload_hash: vec![1; 32],
            payload_size_bytes: 7,
            chunk_count: 2,
            chunk_hashes: vec![vec![2; 32], vec![3; 32]],
            created_at: 99,
        };

        TemplateChunkSetStateStore::upsert(release(), record.clone());

        assert_eq!(TemplateChunkSetStateStore::get(&release()), Some(record));
    }

    #[test]
    fn chunk_store_round_trip() {
        TemplateChunkStore::clear_for_test();
        let chunk_key = TemplateChunkKey::new(release(), 0);
        let record = TemplateChunkRecord {
            bytes: vec![1, 2, 3],
        };

        TemplateChunkStore::upsert(chunk_key.clone(), record.clone());

        assert_eq!(TemplateChunkStore::get(&chunk_key), Some(record));
    }
}
