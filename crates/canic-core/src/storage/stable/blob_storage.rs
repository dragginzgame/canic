//! Module: storage::stable::blob_storage
//!
//! Responsibility: define stable-memory schemas for blob-storage lifecycle state.
//! Does not own: lifecycle policy, endpoint authorization, or DTO conversion.
//! Boundary: ops wrap these records before workflow or public API access.

use crate::{
    cdk::{
        structures::{DefaultMemoryImpl, memory::VirtualMemory},
        types::BoundedString128,
    },
    eager_static,
    model::blob_storage::BlobRootHash,
    storage::{
        prelude::*,
        stable::memory::blob_storage::{
            BLOB_DELETION_PENDING_ID, STORAGE_GATEWAY_PRINCIPALS_ID, STORED_BLOBS_ID,
        },
    },
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::cell::RefCell;

pub const BLOB_STORAGE_SCHEMA_VERSION: u32 = 1;

struct StoredBlobStore;
struct BlobDeletionPendingStore;
struct StorageGatewayPrincipalStore;

eager_static! {
    static STORED_BLOBS: RefCell<
        StableBtreeMap<BlobRootHashKey, StoredBlobRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.blob_storage.stored_blobs.v1", StoredBlobStore, STORED_BLOBS_ID)),
    );
}

eager_static! {
    static BLOB_DELETION_PENDING: RefCell<
        StableBtreeMap<BlobRootHashKey, BlobDeletionPendingRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.blob_storage.deletion_pending.v1", BlobDeletionPendingStore, BLOB_DELETION_PENDING_ID)),
    );
}

eager_static! {
    static STORAGE_GATEWAY_PRINCIPALS: RefCell<
        StableBtreeMap<Principal, StorageGatewayPrincipalRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.blob_storage.gateway_principals.v1", StorageGatewayPrincipalStore, STORAGE_GATEWAY_PRINCIPALS_ID)),
    );
}

///
/// BlobRootHashKey
///
/// Stable key for canonical Toko/Caffeine `sha256:<64-lowercase-hex>` roots.
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BlobRootHashKey {
    pub value: BoundedString128,
}

impl BlobRootHashKey {
    pub const STORABLE_MAX_SIZE: u32 = 128;

    #[must_use]
    pub fn from_hash(hash: &BlobRootHash) -> Self {
        Self {
            value: BoundedString128::new(hash.as_str()),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_ref()
    }

    #[cfg(test)]
    pub fn into_hash(self) -> Result<BlobRootHash, crate::model::blob_storage::BlobRootHashError> {
        BlobRootHash::try_from(self.value.0)
    }
}

impl_storable_bounded!(BlobRootHashKey, BlobRootHashKey::STORABLE_MAX_SIZE, false);

///
/// StoredBlobRecord
///
/// Stable live-blob record keyed by canonical `BlobRootHashKey`.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredBlobRecord {
    pub schema_version: u32,
    pub root_hash: BoundedString128,
    pub registered_at_ns: u64,
}

impl StoredBlobRecord {
    pub const STORABLE_MAX_SIZE: u32 = 160;

    #[must_use]
    pub fn new(root_hash: &BlobRootHash, registered_at_ns: u64) -> Self {
        Self {
            schema_version: BLOB_STORAGE_SCHEMA_VERSION,
            root_hash: BoundedString128::new(root_hash.as_str()),
            registered_at_ns,
        }
    }
}

impl_storable_bounded!(StoredBlobRecord, StoredBlobRecord::STORABLE_MAX_SIZE, false);

///
/// BlobDeletionPendingRecord
///
/// Canonical pending-deletion record for a live blob awaiting gateway scrubber confirmation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobDeletionPendingRecord {
    pub schema_version: u32,
    pub root_hash: BoundedString128,
    pub marked_at_ns: u64,
}

impl BlobDeletionPendingRecord {
    pub const STORABLE_MAX_SIZE: u32 = 160;

    #[must_use]
    pub fn new(root_hash: &BlobRootHash, marked_at_ns: u64) -> Self {
        Self {
            schema_version: BLOB_STORAGE_SCHEMA_VERSION,
            root_hash: BoundedString128::new(root_hash.as_str()),
            marked_at_ns,
        }
    }
}

impl_storable_bounded!(
    BlobDeletionPendingRecord,
    BlobDeletionPendingRecord::STORABLE_MAX_SIZE,
    false
);

///
/// StorageGatewayPrincipalRecord
///
/// Stable authorized immutable-storage gateway principal record.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StorageGatewayPrincipalRecord {
    pub schema_version: u32,
    pub gateway_principal: Principal,
    pub inserted_at_ns: u64,
}

impl StorageGatewayPrincipalRecord {
    pub const STORABLE_MAX_SIZE: u32 = 96;

    #[must_use]
    pub const fn new(gateway_principal: Principal, inserted_at_ns: u64) -> Self {
        Self {
            schema_version: BLOB_STORAGE_SCHEMA_VERSION,
            gateway_principal,
            inserted_at_ns,
        }
    }
}

impl_storable_bounded!(
    StorageGatewayPrincipalRecord,
    StorageGatewayPrincipalRecord::STORABLE_MAX_SIZE,
    false
);

///
/// BlobStorageData
///
/// Canonical stable snapshot for blob-storage lifecycle state.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg(test)]
pub struct BlobStorageData {
    pub stored_blobs: Vec<(BlobRootHashKey, StoredBlobRecord)>,
    pub deletion_pending: Vec<(BlobRootHashKey, BlobDeletionPendingRecord)>,
    pub gateway_principals: Vec<(Principal, StorageGatewayPrincipalRecord)>,
}

///
/// BlobStorageStore
///
/// Stable-memory backing store for non-billing blob-storage lifecycle state.
///

pub struct BlobStorageStore;

impl BlobStorageStore {
    #[must_use]
    pub(crate) fn get_stored_blob(hash: &BlobRootHash) -> Option<StoredBlobRecord> {
        STORED_BLOBS.with_borrow(|map| map.get(&BlobRootHashKey::from_hash(hash)))
    }

    pub(crate) fn upsert_stored_blob(
        hash: &BlobRootHash,
        record: StoredBlobRecord,
    ) -> Option<StoredBlobRecord> {
        STORED_BLOBS.with_borrow_mut(|map| map.insert(BlobRootHashKey::from_hash(hash), record))
    }

    pub(crate) fn remove_stored_blob(hash: &BlobRootHash) -> Option<StoredBlobRecord> {
        STORED_BLOBS.with_borrow_mut(|map| map.remove(&BlobRootHashKey::from_hash(hash)))
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn stored_blobs() -> Vec<(BlobRootHashKey, StoredBlobRecord)> {
        STORED_BLOBS.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    #[must_use]
    pub(crate) fn get_pending_deletion(hash: &BlobRootHash) -> Option<BlobDeletionPendingRecord> {
        BLOB_DELETION_PENDING.with_borrow(|map| map.get(&BlobRootHashKey::from_hash(hash)))
    }

    pub(crate) fn upsert_pending_deletion(
        hash: &BlobRootHash,
        record: BlobDeletionPendingRecord,
    ) -> Option<BlobDeletionPendingRecord> {
        BLOB_DELETION_PENDING
            .with_borrow_mut(|map| map.insert(BlobRootHashKey::from_hash(hash), record))
    }

    pub(crate) fn remove_pending_deletion(
        hash: &BlobRootHash,
    ) -> Option<BlobDeletionPendingRecord> {
        BLOB_DELETION_PENDING.with_borrow_mut(|map| map.remove(&BlobRootHashKey::from_hash(hash)))
    }

    #[must_use]
    pub(crate) fn pending_deletions() -> Vec<(BlobRootHashKey, BlobDeletionPendingRecord)> {
        BLOB_DELETION_PENDING.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    #[must_use]
    pub(crate) fn get_gateway_principal(
        principal: Principal,
    ) -> Option<StorageGatewayPrincipalRecord> {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow(|map| map.get(&principal))
    }

    pub(crate) fn upsert_gateway_principal(
        principal: Principal,
        record: StorageGatewayPrincipalRecord,
    ) -> Option<StorageGatewayPrincipalRecord> {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(|map| map.insert(principal, record))
    }

    pub(crate) fn remove_gateway_principal(
        principal: Principal,
    ) -> Option<StorageGatewayPrincipalRecord> {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(|map| map.remove(&principal))
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn gateway_principals() -> Vec<(Principal, StorageGatewayPrincipalRecord)> {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow(|map| {
            map.iter()
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn export() -> BlobStorageData {
        BlobStorageData {
            stored_blobs: Self::stored_blobs(),
            deletion_pending: Self::pending_deletions(),
            gateway_principals: Self::gateway_principals(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: BlobStorageData) {
        Self::clear();
        STORED_BLOBS.with_borrow_mut(|map| {
            for (key, record) in data.stored_blobs {
                map.insert(key, record);
            }
        });
        BLOB_DELETION_PENDING.with_borrow_mut(|map| {
            for (key, record) in data.deletion_pending {
                map.insert(key, record);
            }
        });
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(|map| {
            for (principal, record) in data.gateway_principals {
                map.insert(principal, record);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        STORED_BLOBS.with_borrow_mut(StableBtreeMap::clear_new);
        BLOB_DELETION_PENDING.with_borrow_mut(StableBtreeMap::clear_new);
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(StableBtreeMap::clear_new);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(value: &str) -> BlobRootHash {
        BlobRootHash::try_from(value).expect("valid blob root hash")
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn h1() -> BlobRootHash {
        hash("sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
    }

    fn h2() -> BlobRootHash {
        hash("sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB")
    }

    #[test]
    fn blob_root_hash_key_uses_normalized_text() {
        let key = BlobRootHashKey::from_hash(&h1());

        assert_eq!(
            key.as_str(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(key.into_hash().expect("hash parses"), h1());
    }

    #[test]
    fn stored_blob_records_round_trip_through_stable_data_snapshot() {
        BlobStorageStore::clear();
        let h1 = h1();
        let h2 = h2();
        let gateway = p(7);

        BlobStorageStore::upsert_stored_blob(&h1, StoredBlobRecord::new(&h1, 10));
        BlobStorageStore::upsert_stored_blob(&h2, StoredBlobRecord::new(&h2, 20));
        BlobStorageStore::upsert_pending_deletion(&h2, BlobDeletionPendingRecord::new(&h2, 30));
        BlobStorageStore::upsert_gateway_principal(
            gateway,
            StorageGatewayPrincipalRecord::new(gateway, 40),
        );

        let exported = BlobStorageStore::export();
        BlobStorageStore::clear();
        assert_eq!(BlobStorageStore::export(), BlobStorageData::default());

        BlobStorageStore::import(exported.clone());

        assert_eq!(BlobStorageStore::export(), exported);
        assert_eq!(
            BlobStorageStore::get_stored_blob(&h1),
            Some(StoredBlobRecord::new(&h1, 10))
        );
        assert_eq!(
            BlobStorageStore::get_pending_deletion(&h2),
            Some(BlobDeletionPendingRecord::new(&h2, 30))
        );
        assert_eq!(
            BlobStorageStore::get_gateway_principal(gateway),
            Some(StorageGatewayPrincipalRecord::new(gateway, 40))
        );
    }

    #[test]
    fn removal_is_idempotent_for_absent_records() {
        BlobStorageStore::clear();
        let hash = h1();
        let gateway = p(9);

        assert_eq!(BlobStorageStore::remove_stored_blob(&hash), None);
        assert_eq!(BlobStorageStore::remove_pending_deletion(&hash), None);
        assert_eq!(BlobStorageStore::remove_gateway_principal(gateway), None);
    }
}
