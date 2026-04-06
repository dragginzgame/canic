use crate::ids::{TemplateChunkKey, TemplateReleaseKey};
use canic_cdk::structures::{
    BTreeMap, DefaultMemoryImpl, Vec as StableVec,
    memory::VirtualMemory,
    storable::{Bound, Storable},
};
use canic_core::CANIC_WASM_CHUNK_BYTES;
use canic_memory::{eager_static, ic_memory, impl_storable_unbounded};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell, collections::BTreeMap as StdBTreeMap};

const TEMPLATE_CHUNK_SETS_ID: u8 = 11;
const TEMPLATE_CHUNK_REFS_ID: u8 = 12;
const TEMPLATE_CHUNK_PAYLOADS_ID: u8 = 61;
const TEMPLATE_CHUNK_REF_RECORD_BYTES: usize = 12;
const TEMPLATE_CHUNK_REF_RECORD_MAX_BYTES: u32 = 12;
const TEMPLATE_CHUNK_PAYLOAD_MAX_BYTES: u32 = 1_048_576;
const _: () = assert!(CANIC_WASM_CHUNK_BYTES == TEMPLATE_CHUNK_PAYLOAD_MAX_BYTES as usize);

struct TemplateChunkRefStore;
struct TemplateChunkPayloadStore;

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
    static TEMPLATE_CHUNK_REFS: RefCell<
        BTreeMap<TemplateChunkKey, TemplateChunkRefRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(TemplateChunkRefStore, TEMPLATE_CHUNK_REFS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNK_PAYLOADS_MEMORY: VirtualMemory<DefaultMemoryImpl> =
        ic_memory!(TemplateChunkPayloadStore, TEMPLATE_CHUNK_PAYLOADS_ID);
}

eager_static! {
    static TEMPLATE_CHUNK_PAYLOADS: RefCell<TemplateChunkPayloadVec> =
        RefCell::new(init_chunk_payloads());
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkRecord {
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplateChunkRefRecord {
    pub slot: u64,
    pub payload_len: u32,
}

impl Storable for TemplateChunkRefRecord {
    const BOUND: Bound = Bound::Bounded {
        max_size: TEMPLATE_CHUNK_REF_RECORD_MAX_BYTES,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.clone().into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(TEMPLATE_CHUNK_REF_RECORD_BYTES);
        bytes.extend_from_slice(&self.slot.to_le_bytes());
        bytes.extend_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let bytes = bytes.as_ref();
        assert_eq!(
            bytes.len(),
            TEMPLATE_CHUNK_REF_RECORD_BYTES,
            "template chunk ref record length mismatch"
        );

        let slot = u64::from_le_bytes(bytes[0..8].try_into().expect("template chunk ref slot"));
        let payload_len = u32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .expect("template chunk ref payload len"),
        );

        Self { slot, payload_len }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TemplateChunkPayloadRecord {
    pub bytes: Vec<u8>,
}

impl Storable for TemplateChunkPayloadRecord {
    const BOUND: Bound = Bound::Bounded {
        max_size: TEMPLATE_CHUNK_PAYLOAD_MAX_BYTES,
        is_fixed_size: false,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.bytes.as_slice())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self {
            bytes: bytes.into_owned(),
        }
    }
}

type TemplateChunkPayloadVec =
    StableVec<TemplateChunkPayloadRecord, VirtualMemory<DefaultMemoryImpl>>;

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
        let payload_len = u32::try_from(record.bytes.len()).unwrap_or(u32::MAX);
        let next_bytes = chunk_entry_size(&chunk_key, payload_len);
        let payload_record = TemplateChunkPayloadRecord {
            bytes: record.bytes,
        };
        let previous = TEMPLATE_CHUNK_REFS.with_borrow(|map| map.get(&chunk_key));
        let previous_bytes = previous.as_ref().map_or(0, |previous| {
            chunk_entry_size(&chunk_key, previous.payload_len)
        });

        let slot = if let Some(previous) = previous.as_ref() {
            TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                payloads.set(previous.slot, &payload_record);
            });
            previous.slot
        } else {
            TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                let slot = payloads.len();
                payloads.push(&payload_record);
                slot
            })
        };

        TEMPLATE_CHUNK_REFS.with_borrow_mut(|map| {
            map.insert(chunk_key, TemplateChunkRefRecord { slot, payload_len });
        });
        canic_core::perf!("chunk_store_insert");

        TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
            if let Some(current) = occupied.as_mut() {
                *current = current
                    .saturating_sub(previous_bytes)
                    .saturating_add(next_bytes);
            }
        });
        canic_core::perf!("chunk_store_accounting");
    }

    // Fetch one template chunk, if present.
    #[must_use]
    pub fn get(chunk_key: &TemplateChunkKey) -> Option<TemplateChunkRecord> {
        TEMPLATE_CHUNK_REFS.with_borrow(|map| {
            map.get(chunk_key).and_then(|chunk_ref| {
                TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                    payloads
                        .get(chunk_ref.slot)
                        .map(|payload| TemplateChunkRecord {
                            bytes: payload.bytes,
                        })
                })
            })
        })
    }

    // Export the full chunk snapshot for ops-owned accounting.
    #[must_use]
    pub fn export() -> Vec<(TemplateChunkKey, TemplateChunkRecord)> {
        TEMPLATE_CHUNK_REFS.with_borrow(|map| {
            map.iter()
                .filter_map(|entry| {
                    TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                        payloads.get(entry.value().slot).map(|payload| {
                            (
                                entry.key().clone(),
                                TemplateChunkRecord {
                                    bytes: payload.bytes,
                                },
                            )
                        })
                    })
                })
                .collect()
        })
    }

    // Return current chunk occupied bytes without cloning the full chunk snapshot.
    #[must_use]
    pub fn occupied_bytes() -> u64 {
        if let Some(bytes) = TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow(|occupied| *occupied) {
            bytes
        } else {
            let bytes = TEMPLATE_CHUNK_REFS.with_borrow(|map| {
                map.iter()
                    .map(|entry| chunk_entry_size(entry.key(), entry.value().payload_len))
                    .sum()
            });
            TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| {
                *occupied = Some(bytes);
            });
            bytes
        }
    }

    // Return current chunk bytes for one key.
    #[must_use]
    pub fn entry_bytes(chunk_key: &TemplateChunkKey) -> Option<u64> {
        TEMPLATE_CHUNK_REFS.with_borrow(|map| {
            map.get(chunk_key)
                .map(|chunk_ref| chunk_entry_size(chunk_key, chunk_ref.payload_len))
        })
    }

    // Count staged chunks by release without cloning chunk payload bytes.
    #[must_use]
    pub fn count_by_release() -> StdBTreeMap<TemplateReleaseKey, u32> {
        let mut counts: StdBTreeMap<TemplateReleaseKey, u32> = StdBTreeMap::new();

        TEMPLATE_CHUNK_REFS.with_borrow(|map| {
            for entry in map.iter() {
                let count = counts.entry(entry.key().release.clone()).or_insert(0);
                *count = u32::saturating_add(*count, 1);
            }
        });

        counts
    }

    // Clear the chunk store.
    pub fn clear() {
        TEMPLATE_CHUNK_REFS.with_borrow_mut(BTreeMap::clear);
        TEMPLATE_CHUNK_PAYLOADS.with_borrow_mut(|payloads| {
            *payloads = reset_chunk_payloads();
        });
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

fn init_chunk_payloads() -> TemplateChunkPayloadVec {
    TEMPLATE_CHUNK_PAYLOADS_MEMORY.with(|memory| StableVec::init(memory.clone()))
}

fn reset_chunk_payloads() -> TemplateChunkPayloadVec {
    TEMPLATE_CHUNK_PAYLOADS_MEMORY.with(|memory| StableVec::new(memory.clone()))
}

fn chunk_entry_size(chunk_key: &TemplateChunkKey, payload_len: u32) -> u64 {
    (chunk_key.to_bytes().len() + TEMPLATE_CHUNK_REF_RECORD_BYTES + payload_len as usize) as u64
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

    #[test]
    fn chunk_store_overwrite_reuses_slot() {
        TemplateChunkStore::clear_for_test();
        let chunk_key = TemplateChunkKey::new(release(), 0);
        TemplateChunkStore::upsert(
            chunk_key.clone(),
            TemplateChunkRecord {
                bytes: vec![7, 8, 9],
            },
        );
        TemplateChunkStore::upsert(chunk_key.clone(), TemplateChunkRecord { bytes: vec![4, 5] });

        let payload_slots =
            TEMPLATE_CHUNK_PAYLOADS.with_borrow(canic_cdk::structures::StableVec::len);
        assert_eq!(payload_slots, 1);
        assert_eq!(
            TemplateChunkStore::get(&chunk_key),
            Some(TemplateChunkRecord { bytes: vec![4, 5] })
        );
        assert_eq!(
            TemplateChunkStore::entry_bytes(&chunk_key),
            Some(chunk_entry_size(&chunk_key, 2))
        );
    }

    #[test]
    fn chunk_store_occupied_bytes_matches_active_entries() {
        TemplateChunkStore::clear_for_test();
        let first_key = TemplateChunkKey::new(release(), 0);
        let second_key = TemplateChunkKey::new(release(), 1);

        TemplateChunkStore::upsert(
            first_key.clone(),
            TemplateChunkRecord {
                bytes: vec![1, 2, 3],
            },
        );
        TemplateChunkStore::upsert(
            second_key.clone(),
            TemplateChunkRecord { bytes: vec![4, 5] },
        );

        let expected = chunk_entry_size(&first_key, 3) + chunk_entry_size(&second_key, 2);
        assert_eq!(TemplateChunkStore::occupied_bytes(), expected);
    }
}
