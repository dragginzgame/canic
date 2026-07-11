//! Module: storage::stable::blob_storage
//!
//! Responsibility: define stable-memory schemas for blob-storage lifecycle state.
//! Does not own: lifecycle policy, endpoint authorization, or DTO conversion.
//! Boundary: ops wrap these records before workflow or public API access.

#![cfg_attr(
    not(feature = "blob-storage"),
    expect(
        dead_code,
        reason = "blob-storage schema remains available to the unconditional state descriptor registry"
    )
)]

#[cfg(feature = "blob-storage")]
use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
#[cfg(feature = "blob-storage")]
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    eager_static,
    model::blob_storage::BlobRootHash,
    role_contract::allocation::memory::blob_storage::{
        BLOB_DELETION_PENDING_ID, STORAGE_GATEWAY_PRINCIPALS_ID, STORED_BLOBS_ID,
    },
};
use crate::{cdk::types::BoundedString128, storage::prelude::*};
#[cfg(feature = "blob-storage")]
use std::cell::RefCell;

#[cfg(feature = "blob-storage-billing")]
use crate::cdk::structures::cell::Cell;

#[cfg(feature = "blob-storage-billing")]
use crate::role_contract::allocation::memory::blob_storage::BLOB_STORAGE_BILLING_ID;

pub const BLOB_STORAGE_SCHEMA_VERSION: u32 = 1;

#[cfg(feature = "blob-storage")]
struct StoredBlobStore;
#[cfg(feature = "blob-storage")]
struct BlobDeletionPendingStore;
#[cfg(feature = "blob-storage")]
struct StorageGatewayPrincipalStore;
#[cfg(feature = "blob-storage-billing")]
struct BlobStorageBillingStore;

#[cfg(feature = "blob-storage")]
eager_static! {
    static STORED_BLOBS: RefCell<
        StableBtreeMap<BlobRootHashKey, StoredBlobRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.blob_storage.stored_blobs.v1", ty = StoredBlobStore, id = STORED_BLOBS_ID)),
    );
}

#[cfg(feature = "blob-storage")]
eager_static! {
    static BLOB_DELETION_PENDING: RefCell<
        StableBtreeMap<BlobRootHashKey, BlobDeletionPendingRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.blob_storage.deletion_pending.v1", ty = BlobDeletionPendingStore, id = BLOB_DELETION_PENDING_ID)),
    );
}

#[cfg(feature = "blob-storage")]
eager_static! {
    static STORAGE_GATEWAY_PRINCIPALS: RefCell<
        StableBtreeMap<Principal, StorageGatewayPrincipalRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.blob_storage.gateway_principals.v1", ty = StorageGatewayPrincipalStore, id = STORAGE_GATEWAY_PRINCIPALS_ID)),
    );
}

#[cfg(feature = "blob-storage-billing")]
eager_static! {
    static BLOB_STORAGE_BILLING: RefCell<
        Cell<BlobStorageBillingStateRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.blob_storage.billing.v1", ty = BlobStorageBillingStore, id = BLOB_STORAGE_BILLING_ID),
        BlobStorageBillingStateRecord::default(),
    ));
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
    #[cfg(feature = "blob-storage")]
    pub fn from_hash(hash: &BlobRootHash) -> Self {
        Self {
            value: BoundedString128::new(hash.as_str()),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_ref()
    }

    #[cfg(all(test, feature = "blob-storage"))]
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
    pub const STATE_CONTRACT_NAME: &'static str = "StoredBlobRecord";
    pub const STORABLE_MAX_SIZE: u32 = 160;

    #[must_use]
    #[cfg(feature = "blob-storage")]
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
    pub const STATE_CONTRACT_NAME: &'static str = "BlobDeletionPendingRecord";
    pub const STORABLE_MAX_SIZE: u32 = 160;

    #[must_use]
    #[cfg(feature = "blob-storage")]
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
    pub const STATE_CONTRACT_NAME: &'static str = "StorageGatewayPrincipalRecord";
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
/// BlobStorageBillingConfigRecord
///
/// Stable blob-storage billing configuration record.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageBillingConfigRecord {
    pub schema_version: u32,
    pub cashier_canister_id: Principal,
    pub project_cycles_reserve: u128,
    pub min_upload_balance: u128,
    pub target_upload_balance: u128,
    pub gateway_principal_limit: u64,
    pub updated_at_ns: u64,
}

impl_storable_bounded!(
    BlobStorageBillingConfigRecord,
    BlobStorageBillingConfigRecord::STORABLE_MAX_SIZE,
    false
);

impl BlobStorageBillingConfigRecord {
    pub const STORABLE_MAX_SIZE: u32 = 192;

    #[must_use]
    #[cfg(feature = "blob-storage-billing")]
    pub const fn new(
        cashier_canister_id: Principal,
        project_cycles_reserve: u128,
        min_upload_balance: u128,
        target_upload_balance: u128,
        gateway_principal_limit: u64,
        updated_at_ns: u64,
    ) -> Self {
        Self {
            schema_version: BLOB_STORAGE_SCHEMA_VERSION,
            cashier_canister_id,
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
            gateway_principal_limit,
            updated_at_ns,
        }
    }
}

///
/// BlobStorageBillingStateRecord
///
/// Stable singleton state for blob-storage billing.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageBillingStateRecord {
    pub schema_version: u32,
    pub config: Option<BlobStorageBillingConfigRecord>,
    pub last_gateway_principal_sync_at_ns: Option<u64>,
}

impl Default for BlobStorageBillingStateRecord {
    fn default() -> Self {
        Self {
            schema_version: BLOB_STORAGE_SCHEMA_VERSION,
            config: None,
            last_gateway_principal_sync_at_ns: None,
        }
    }
}

impl_storable_bounded!(
    BlobStorageBillingStateRecord,
    BlobStorageBillingStateRecord::STORABLE_MAX_SIZE,
    false
);

impl BlobStorageBillingStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "BlobStorageBillingStateRecord";
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

///
/// StoredBlobEntryRecord
///
/// One logical stored-blob snapshot row preserving its stable key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredBlobEntryRecord {
    pub key: BlobRootHashKey,
    pub record: StoredBlobRecord,
}

///
/// StoredBlobsData
///
/// Canonical stored-blob allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoredBlobsData {
    pub entries: Vec<StoredBlobEntryRecord>,
}

impl StoredBlobsData {
    pub const STATE_CONTRACT_NAME: &'static str = "StoredBlobsData";
}

///
/// BlobDeletionPendingEntryRecord
///
/// One logical pending-deletion snapshot row preserving its stable key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlobDeletionPendingEntryRecord {
    pub key: BlobRootHashKey,
    pub record: BlobDeletionPendingRecord,
}

///
/// BlobDeletionPendingData
///
/// Canonical pending-deletion allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlobDeletionPendingData {
    pub entries: Vec<BlobDeletionPendingEntryRecord>,
}

impl BlobDeletionPendingData {
    pub const STATE_CONTRACT_NAME: &'static str = "BlobDeletionPendingData";
}

///
/// StorageGatewayPrincipalsData
///
/// Canonical storage-gateway-principal allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StorageGatewayPrincipalsData {
    pub entries: Vec<StorageGatewayPrincipalRecord>,
}

impl StorageGatewayPrincipalsData {
    pub const STATE_CONTRACT_NAME: &'static str = "StorageGatewayPrincipalsData";
}

///
/// BlobStorageBillingStateData
///
/// Canonical blob-storage billing allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlobStorageBillingStateData {
    pub state: BlobStorageBillingStateRecord,
}

impl BlobStorageBillingStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "BlobStorageBillingStateData";
}

///
/// BlobStorageStore
///
/// Stable-memory backing store for non-billing blob-storage lifecycle state.
///

#[cfg(feature = "blob-storage")]
pub struct BlobStorageStore;

#[cfg(feature = "blob-storage")]
impl BlobStorageStore {
    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub(crate) fn billing_config() -> Option<BlobStorageBillingConfigRecord> {
        Self::billing_state_data().state.config
    }

    #[cfg(feature = "blob-storage-billing")]
    pub(crate) fn set_billing_config(config: BlobStorageBillingConfigRecord) {
        BLOB_STORAGE_BILLING.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            state.schema_version = BLOB_STORAGE_SCHEMA_VERSION;
            state.config = Some(config);
            cell.set(state);
        });
    }

    #[cfg(feature = "blob-storage-billing")]
    pub(crate) fn set_last_gateway_principal_sync_at_ns(now_ns: u64) {
        BLOB_STORAGE_BILLING.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            state.schema_version = BLOB_STORAGE_SCHEMA_VERSION;
            state.last_gateway_principal_sync_at_ns = Some(now_ns);
            cell.set(state);
        });
    }

    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub(crate) fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        Self::billing_state_data()
            .state
            .last_gateway_principal_sync_at_ns
    }

    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub(crate) fn billing_state_data() -> BlobStorageBillingStateData {
        BlobStorageBillingStateData {
            state: BLOB_STORAGE_BILLING.with_borrow(|cell| cell.get().clone()),
        }
    }

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
    pub(crate) fn stored_blob_count() -> u64 {
        STORED_BLOBS.with_borrow(StableBtreeMap::len)
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn stored_blobs_data() -> StoredBlobsData {
        StoredBlobsData {
            entries: STORED_BLOBS.with_borrow(|map| {
                map.iter()
                    .map(|entry| StoredBlobEntryRecord {
                        key: entry.key().clone(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
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
    pub(crate) fn pending_deletion_count() -> u64 {
        BLOB_DELETION_PENDING.with_borrow(StableBtreeMap::len)
    }

    #[must_use]
    pub(crate) fn pending_deletions_data() -> BlobDeletionPendingData {
        BlobDeletionPendingData {
            entries: BLOB_DELETION_PENDING.with_borrow(|map| {
                map.iter()
                    .map(|entry| BlobDeletionPendingEntryRecord {
                        key: entry.key().clone(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
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
    pub(crate) fn gateway_principal_count() -> u64 {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow(StableBtreeMap::len)
    }

    #[must_use]
    pub(crate) fn gateway_principals_data() -> StorageGatewayPrincipalsData {
        StorageGatewayPrincipalsData {
            entries: STORAGE_GATEWAY_PRINCIPALS
                .with_borrow(|map| map.iter().map(|entry| entry.value()).collect()),
        }
    }

    #[cfg(test)]
    pub(crate) fn import_stored_blobs(data: StoredBlobsData) {
        STORED_BLOBS.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.key, entry.record);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn import_pending_deletions(data: BlobDeletionPendingData) {
        BLOB_DELETION_PENDING.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.key, entry.record);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn import_gateway_principals(data: StorageGatewayPrincipalsData) {
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(|map| {
            map.clear_new();
            for record in data.entries {
                map.insert(record.gateway_principal, record);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn clear() {
        STORED_BLOBS.with_borrow_mut(StableBtreeMap::clear_new);
        BLOB_DELETION_PENDING.with_borrow_mut(StableBtreeMap::clear_new);
        STORAGE_GATEWAY_PRINCIPALS.with_borrow_mut(StableBtreeMap::clear_new);
    }

    #[cfg(all(test, feature = "blob-storage-billing"))]
    pub(crate) fn clear_billing() {
        BLOB_STORAGE_BILLING
            .with_borrow_mut(|cell| cell.set(BlobStorageBillingStateRecord::default()));
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(all(test, feature = "blob-storage"))]
mod tests {
    use super::*;

    fn hash(value: &str) -> BlobRootHash {
        BlobRootHash::try_from(value).expect("valid blob root hash")
    }

    const fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn h1() -> BlobRootHash {
        hash("sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
    }

    fn h1_lower() -> BlobRootHash {
        hash("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
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
    fn stable_blob_maps_use_normalized_root_hash_keys() {
        BlobStorageStore::clear();
        let upper = h1();
        let lower = h1_lower();

        assert_eq!(upper, lower);

        BlobStorageStore::upsert_stored_blob(&upper, StoredBlobRecord::new(&upper, 10));
        BlobStorageStore::upsert_stored_blob(&lower, StoredBlobRecord::new(&lower, 20));

        assert_eq!(BlobStorageStore::stored_blob_count(), 1);
        assert_eq!(
            BlobStorageStore::get_stored_blob(&upper),
            Some(StoredBlobRecord::new(&lower, 20))
        );

        BlobStorageStore::upsert_pending_deletion(
            &upper,
            BlobDeletionPendingRecord::new(&upper, 30),
        );
        BlobStorageStore::upsert_pending_deletion(
            &lower,
            BlobDeletionPendingRecord::new(&lower, 40),
        );

        assert_eq!(BlobStorageStore::pending_deletion_count(), 1);
        assert_eq!(
            BlobStorageStore::get_pending_deletion(&upper),
            Some(BlobDeletionPendingRecord::new(&lower, 40))
        );
        assert_eq!(
            BlobStorageStore::pending_deletions_data()
                .entries
                .into_iter()
                .map(|entry| entry.key.as_str().to_string())
                .collect::<Vec<_>>(),
            vec![lower.as_str().to_string()]
        );
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

        assert_eq!(BlobStorageStore::stored_blob_count(), 2);
        assert_eq!(BlobStorageStore::pending_deletion_count(), 1);
        assert_eq!(BlobStorageStore::gateway_principal_count(), 1);

        let stored_blobs = BlobStorageStore::stored_blobs_data();
        let pending_deletions = BlobStorageStore::pending_deletions_data();
        let gateway_principals = BlobStorageStore::gateway_principals_data();
        BlobStorageStore::clear();
        assert_eq!(
            BlobStorageStore::stored_blobs_data(),
            StoredBlobsData::default()
        );
        assert_eq!(
            BlobStorageStore::pending_deletions_data(),
            BlobDeletionPendingData::default()
        );
        assert_eq!(
            BlobStorageStore::gateway_principals_data(),
            StorageGatewayPrincipalsData::default()
        );
        assert_eq!(BlobStorageStore::stored_blob_count(), 0);
        assert_eq!(BlobStorageStore::pending_deletion_count(), 0);
        assert_eq!(BlobStorageStore::gateway_principal_count(), 0);

        BlobStorageStore::import_stored_blobs(stored_blobs.clone());
        BlobStorageStore::import_pending_deletions(pending_deletions.clone());
        BlobStorageStore::import_gateway_principals(gateway_principals.clone());

        assert_eq!(BlobStorageStore::stored_blobs_data(), stored_blobs);
        assert_eq!(
            BlobStorageStore::pending_deletions_data(),
            pending_deletions
        );
        assert_eq!(
            BlobStorageStore::gateway_principals_data(),
            gateway_principals
        );
        assert_eq!(BlobStorageStore::stored_blob_count(), 2);
        assert_eq!(BlobStorageStore::pending_deletion_count(), 1);
        assert_eq!(BlobStorageStore::gateway_principal_count(), 1);
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

    #[cfg(feature = "blob-storage-billing")]
    #[test]
    fn billing_state_exports_through_canonical_data_snapshot() {
        BlobStorageStore::clear_billing();
        assert_eq!(
            BlobStorageStore::billing_state_data(),
            BlobStorageBillingStateData::default()
        );

        let cashier = p(8);
        BlobStorageStore::set_billing_config(BlobStorageBillingConfigRecord::new(
            cashier, 10, 20, 30, 40, 50,
        ));
        BlobStorageStore::set_last_gateway_principal_sync_at_ns(60);

        let data = BlobStorageStore::billing_state_data();
        let config = data.state.config.expect("billing config");
        assert_eq!(config.cashier_canister_id, cashier);
        assert_eq!(data.state.last_gateway_principal_sync_at_ns, Some(60));
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
