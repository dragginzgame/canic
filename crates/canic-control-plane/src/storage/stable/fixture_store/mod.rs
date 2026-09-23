//! Durable opaque fixture objects, chunk payloads and exact target grant revisions.
//!
//! This namespace is disjoint from executable templates. Operations validate
//! content and capacity before committing these synchronous local mutations.

use candid::{CandidType, Deserialize, Principal};
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, btreemap::BTreeMap, memory::RuntimeMemory},
    dto::fixture_provisioning::{FixtureDescriptor, FixtureGrant},
    role_contract::allocation::memory::control_plane::FIXTURE_STORE_ID,
};
use std::cell::RefCell;

// Tag, content/principal digest, and chunk ordinal form a fixed namespace key.
const KEY_BYTES: usize = 37;
const ACCOUNTING_KEY: [u8; KEY_BYTES] = [0; KEY_BYTES];

std::thread_local! {
    static FIXTURES: RefCell<BTreeMap<[u8; KEY_BYTES], Vec<u8>, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.fixture_store.v1",
            ty = FixtureStore,
            id = FIXTURE_STORE_ID
        )));
}

/// Exact descriptor and monotonic ingestion progress, committed with each chunk.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct FixtureSourceRecord {
    pub descriptor: FixtureDescriptor,
    pub next_chunk: u32,
    pub received_bytes: u64,
}

/// Stable namespace entry; tags distinguish content records, chunks and grants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureStoreEntryRecord {
    pub key: [u8; KEY_BYTES],
    pub value: Vec<u8>,
}

impl FixtureStoreEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FixtureStoreEntryRecord";
}

/// Canonical same-release snapshot of this complete allocation, including its ledger.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FixtureStoreData {
    pub entries: Vec<FixtureStoreEntryRecord>,
}

impl FixtureStoreData {
    pub const STATE_CONTRACT_NAME: &'static str = "FixtureStoreData";
}

/// Authoritative stable map; all writes account for exact retained key/value bytes.
pub struct FixtureStore;

impl FixtureStore {
    /// Count metadata and chunk keys without reading chunk payloads.
    pub fn inventory() -> crate::view::fixture_store::FixtureStoreInventoryView {
        FIXTURES.with_borrow(|map| {
            let mut inventory = crate::view::fixture_store::FixtureStoreInventoryView {
                sources: 0,
                expected_chunks: 0,
                stored_chunks: 0,
            };
            for entry in map.iter() {
                match entry.key()[0] {
                    1 => {
                        let source: FixtureSourceRecord =
                            candid::decode_one(&entry.value()).expect("fixture source record");
                        inventory.sources += 1;
                        inventory.expected_chunks = inventory
                            .expected_chunks
                            .checked_add(source.descriptor.chunks.len() as u64)
                            .expect("bounded fixture chunk inventory");
                    }
                    2 => inventory.stored_chunks += 1,
                    _ => {}
                }
            }
            inventory
        })
    }

    pub fn source(content: [u8; 32]) -> Option<FixtureSourceRecord> {
        get(key(1, content, 0))
            .map(|bytes| candid::decode_one(&bytes).expect("fixture source record"))
    }

    pub fn grant(target: Principal) -> Option<FixtureGrant> {
        get(grant_key(target))
            .map(|bytes| candid::decode_one(&bytes).expect("fixture grant record"))
    }

    pub fn chunk(content: [u8; 32], index: u32) -> Option<Vec<u8>> {
        get(key(2, content, index))
    }

    pub fn source_entry(
        content: [u8; 32],
        record: &FixtureSourceRecord,
    ) -> FixtureStoreEntryRecord {
        FixtureStoreEntryRecord {
            key: key(1, content, 0),
            value: candid::encode_one(record).expect("bounded fixture source encoding"),
        }
    }

    pub fn chunk_entry(content: [u8; 32], index: u32, bytes: Vec<u8>) -> FixtureStoreEntryRecord {
        FixtureStoreEntryRecord {
            key: key(2, content, index),
            value: bytes,
        }
    }

    pub fn grant_entry(target: Principal, grant: &FixtureGrant) -> FixtureStoreEntryRecord {
        FixtureStoreEntryRecord {
            key: grant_key(target),
            value: candid::encode_one(grant).expect("bounded fixture grant encoding"),
        }
    }

    /// Project replacements without mutating, including newly introduced keys.
    pub fn projected_bytes(entries: &[FixtureStoreEntryRecord]) -> Option<u64> {
        let mut bytes = Self::occupied_bytes();
        if get(ACCOUNTING_KEY).is_none() && !entries.is_empty() {
            bytes = bytes.checked_add((KEY_BYTES + 8) as u64)?;
        }
        for entry in entries {
            if let Some(previous) = get(entry.key) {
                bytes = bytes.checked_sub(previous.len() as u64)?;
            } else {
                bytes = bytes.checked_add(KEY_BYTES as u64)?;
            }
            bytes = bytes.checked_add(entry.value.len() as u64)?;
        }
        Some(bytes)
    }

    /// Apply an already admitted local transaction. Never split this across an await.
    pub fn commit(entries: Vec<FixtureStoreEntryRecord>, projected_bytes: u64) {
        FIXTURES.with_borrow_mut(|map| {
            for entry in entries {
                map.insert(entry.key, entry.value);
            }
            map.insert(ACCOUNTING_KEY, projected_bytes.to_le_bytes().to_vec());
        });
    }

    /// Remove at most one bounded source/chunk/grant entry and its exact byte charge.
    /// The Store GC owner must already have fenced all reads and writes.
    pub fn clear_retired_step() -> bool {
        FIXTURES.with_borrow_mut(|map| {
            let next = map
                .range((
                    std::ops::Bound::Excluded(ACCOUNTING_KEY),
                    std::ops::Bound::Unbounded,
                ))
                .next()
                .map(|entry| (*entry.key(), entry.value()));
            if let Some((key, value)) = next {
                let bytes = map.get(&ACCOUNTING_KEY).expect("fixture byte ledger");
                let occupied = u64::from_le_bytes(bytes.try_into().expect("fixture byte ledger"));
                let remaining = occupied
                    .checked_sub((KEY_BYTES + value.len()) as u64)
                    .expect("fixture entry charge");
                map.remove(&key);
                map.insert(ACCOUNTING_KEY, remaining.to_le_bytes().to_vec());
            }
            if map.len() <= 1 {
                map.remove(&ACCOUNTING_KEY);
                true
            } else {
                false
            }
        })
    }

    pub fn occupied_bytes() -> u64 {
        get(ACCOUNTING_KEY).map_or(0, |bytes| {
            u64::from_le_bytes(bytes.try_into().expect("fixture byte ledger"))
        })
    }

    #[cfg(test)]
    pub fn export() -> FixtureStoreData {
        FIXTURES.with_borrow(|map| FixtureStoreData {
            entries: map
                .iter()
                .map(|entry| FixtureStoreEntryRecord {
                    key: *entry.key(),
                    value: entry.value(),
                })
                .collect(),
        })
    }

    #[cfg(test)]
    pub fn import(data: FixtureStoreData) {
        FIXTURES.with_borrow_mut(|map| {
            while let Some(entry) = map.first_key_value() {
                map.remove(&entry.0);
            }
            for entry in data.entries {
                map.insert(entry.key, entry.value);
            }
        });
    }
}

fn get(key: [u8; KEY_BYTES]) -> Option<Vec<u8>> {
    FIXTURES.with_borrow(|map| map.get(&key))
}

fn grant_key(target: Principal) -> [u8; KEY_BYTES] {
    // Principal encodings are at most 29 bytes; length prevents padding ambiguity.
    let mut identity = [0; 32];
    identity[0] = u8::try_from(target.as_slice().len()).expect("bounded Principal");
    identity[1..=target.as_slice().len()].copy_from_slice(target.as_slice());
    key(3, identity, 0)
}

fn key(tag: u8, content: [u8; 32], index: u32) -> [u8; KEY_BYTES] {
    let mut key = [0; KEY_BYTES];
    key[0] = tag;
    key[1..33].copy_from_slice(&content);
    key[33..].copy_from_slice(&index.to_be_bytes());
    key
}
