use crate::ids::{TemplateChunkKey, TemplateReleaseKey};
use canic_core::CANIC_WASM_CHUNK_BYTES;
use canic_core::cdk::structures::{
    DefaultMemoryImpl, Vec as StableVec,
    memory::VirtualMemory,
    storable::{Bound, Storable},
};
use canic_core::eager_static;
use canic_core::impl_storable_unbounded;
use canic_core::role_contract::allocation::memory::template::{
    TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID, TEMPLATE_CHUNK_SETS_ID,
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use serde::{Deserialize, Serialize};
#[cfg(feature = "root-control-plane")]
use std::collections::BTreeMap as StdBTreeMap;
use std::{borrow::Cow, cell::RefCell};

const TEMPLATE_CHUNK_REF_RECORD_BYTES: usize = 12;
const TEMPLATE_CHUNK_REF_RECORD_MAX_BYTES: u32 = 12;
const TEMPLATE_CHUNK_PAYLOAD_MAX_BYTES: u32 = 1_048_576;
const _: () = assert!(CANIC_WASM_CHUNK_BYTES == TEMPLATE_CHUNK_PAYLOAD_MAX_BYTES as usize);

struct TemplateChunkRefStore;
struct TemplateChunkPayloadStore;

eager_static! {
    static TEMPLATE_CHUNK_SETS: RefCell<
        StableBtreeMap<TemplateReleaseKey, TemplateChunkSetRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(canic_core::ic_memory_key!("canic.control_plane.template_chunk_sets.v1", TemplateChunkSetStateStore, TEMPLATE_CHUNK_SETS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNK_SETS_OCCUPIED_BYTES: RefCell<Option<u64>> = RefCell::new(None);
}

eager_static! {
    static TEMPLATE_CHUNK_REFS: RefCell<
        StableBtreeMap<TemplateChunkKey, TemplateChunkRefRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(canic_core::ic_memory_key!("canic.control_plane.template_chunk_refs.v1", TemplateChunkRefStore, TEMPLATE_CHUNK_REFS_ID)),
    );
}

eager_static! {
    static TEMPLATE_CHUNK_PAYLOADS_MEMORY: VirtualMemory<DefaultMemoryImpl> =
        canic_core::ic_memory_key!("canic.control_plane.template_chunk_payloads.v1", TemplateChunkPayloadStore, TEMPLATE_CHUNK_PAYLOADS_ID);
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

impl TemplateChunkSetRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkSetRecord";
}

impl_storable_unbounded!(TemplateChunkSetRecord);

///
/// TemplateChunkSetEntryRecord
///
/// One logical chunk-set snapshot row preserving its stable release key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkSetEntryRecord {
    pub release: TemplateReleaseKey,
    pub record: TemplateChunkSetRecord,
}

///
/// TemplateChunkSetsData
///
/// Canonical template-chunk-set allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateChunkSetsData {
    pub entries: Vec<TemplateChunkSetEntryRecord>,
}

impl TemplateChunkSetsData {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkSetsData";
}

///
/// TemplateChunkRecord
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkRecord {
    pub bytes: Vec<u8>,
}

///
/// TemplateChunkRefRecord
///
/// Persisted map value linking one stable chunk key to a payload-vector slot.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkRefRecord {
    pub slot: u64,
    pub payload_len: u32,
}

impl TemplateChunkRefRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkRefRecord";
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

///
/// TemplateChunkPayloadRecord
///
/// Persisted stable-vector value containing one raw template chunk payload.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkPayloadRecord {
    pub bytes: Vec<u8>,
}

impl TemplateChunkPayloadRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkPayloadRecord";
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
/// TemplateChunkRefEntryRecord
///
/// One physical chunk-reference snapshot row preserving its stable chunk key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkRefEntryRecord {
    pub chunk_key: TemplateChunkKey,
    pub record: TemplateChunkRefRecord,
}

///
/// TemplateChunkRefsData
///
/// Canonical template-chunk-reference allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateChunkRefsData {
    pub entries: Vec<TemplateChunkRefEntryRecord>,
}

impl TemplateChunkRefsData {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkRefsData";
}

///
/// TemplateChunkPayloadEntryRecord
///
/// One physical chunk-payload snapshot row preserving its stable vector slot.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateChunkPayloadEntryRecord {
    pub slot: u64,
    pub record: TemplateChunkPayloadRecord,
}

///
/// TemplateChunkPayloadsData
///
/// Canonical template-chunk-payload allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateChunkPayloadsData {
    pub entries: Vec<TemplateChunkPayloadEntryRecord>,
}

impl TemplateChunkPayloadsData {
    pub const STATE_CONTRACT_NAME: &'static str = "TemplateChunkPayloadsData";
}

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
    pub fn export() -> TemplateChunkSetsData {
        TemplateChunkSetsData {
            entries: TEMPLATE_CHUNK_SETS.with_borrow(|map| {
                map.iter()
                    .map(|entry| TemplateChunkSetEntryRecord {
                        release: entry.key().clone(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub fn import(data: TemplateChunkSetsData) {
        Self::clear();
        for entry in data.entries {
            Self::upsert(entry.release, entry.record);
        }
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
        TEMPLATE_CHUNK_SETS.with_borrow_mut(StableBtreeMap::clear_new);
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

    // Count indexed chunks whose payload slots resolve.
    #[must_use]
    pub fn count() -> usize {
        TEMPLATE_CHUNK_REFS.with_borrow(|map| {
            map.iter()
                .filter(|entry| {
                    TEMPLATE_CHUNK_PAYLOADS
                        .with_borrow(|payloads| payloads.get(entry.value().slot).is_some())
                })
                .count()
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
    #[cfg(feature = "root-control-plane")]
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
        TEMPLATE_CHUNK_REFS.with_borrow_mut(StableBtreeMap::clear_new);
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

    #[cfg(test)]
    fn export_refs() -> TemplateChunkRefsData {
        TemplateChunkRefsData {
            entries: TEMPLATE_CHUNK_REFS.with_borrow(|map| {
                map.iter()
                    .map(|entry| {
                        let record = entry.value();
                        TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                            let payload = payloads
                                .get(record.slot)
                                .expect("chunk reference slot must resolve to a payload");
                            assert_eq!(
                                usize::try_from(record.payload_len).unwrap_or(usize::MAX),
                                payload.bytes.len(),
                                "chunk reference payload length must match stored bytes"
                            );
                        });
                        TemplateChunkRefEntryRecord {
                            chunk_key: entry.key().clone(),
                            record,
                        }
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    fn import_refs(data: TemplateChunkRefsData) {
        TEMPLATE_CHUNK_REFS.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.chunk_key, entry.record);
            }
        });
        TEMPLATE_CHUNKS_OCCUPIED_BYTES.with_borrow_mut(|occupied| *occupied = None);
    }

    #[cfg(test)]
    fn export_payloads() -> TemplateChunkPayloadsData {
        TemplateChunkPayloadsData {
            entries: TEMPLATE_CHUNK_PAYLOADS.with_borrow(|payloads| {
                (0..payloads.len())
                    .map(|slot| TemplateChunkPayloadEntryRecord {
                        slot,
                        record: payloads
                            .get(slot)
                            .expect("chunk payload slot must exist within stable vector length"),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    fn import_payloads(data: TemplateChunkPayloadsData) {
        TEMPLATE_CHUNK_PAYLOADS.with_borrow_mut(|payloads| {
            *payloads = reset_chunk_payloads();
            for entry in data.entries {
                assert_eq!(
                    entry.slot,
                    payloads.len(),
                    "chunk payload slots must be contiguous"
                );
                payloads.push(&entry.record);
            }
        });
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
    fn chunk_set_store_round_trips_through_canonical_data_snapshot() {
        TemplateChunkSetStateStore::clear_for_test();
        TemplateChunkSetStateStore::upsert(
            release(),
            TemplateChunkSetRecord {
                payload_hash: vec![1; 32],
                payload_size_bytes: 7,
                chunk_count: 2,
                chunk_hashes: vec![vec![2; 32], vec![3; 32]],
                created_at: 99,
            },
        );

        let data = TemplateChunkSetStateStore::export();
        TemplateChunkSetStateStore::clear_for_test();
        TemplateChunkSetStateStore::import(data.clone());

        assert_eq!(TemplateChunkSetStateStore::export(), data);
        TemplateChunkSetStateStore::clear_for_test();
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
    fn chunk_ref_and_payload_stores_round_trip_exact_physical_snapshots() {
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

        let refs = TemplateChunkStore::export_refs();
        let payloads = TemplateChunkStore::export_payloads();
        TemplateChunkStore::clear_for_test();
        TemplateChunkStore::import_payloads(payloads.clone());
        TemplateChunkStore::import_refs(refs.clone());

        assert_eq!(TemplateChunkStore::export_refs(), refs);
        assert_eq!(TemplateChunkStore::export_payloads(), payloads);
        assert_eq!(
            TemplateChunkStore::get(&first_key),
            Some(TemplateChunkRecord {
                bytes: vec![1, 2, 3]
            })
        );
        assert_eq!(
            TemplateChunkStore::get(&second_key),
            Some(TemplateChunkRecord { bytes: vec![4, 5] })
        );
        TemplateChunkStore::clear_for_test();
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
            TEMPLATE_CHUNK_PAYLOADS.with_borrow(canic_core::cdk::structures::StableVec::len);
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
