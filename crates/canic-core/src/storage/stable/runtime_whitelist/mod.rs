//! Module: storage::stable::runtime_whitelist
//!
//! Responsibility: persist the sole canonical managed-role runtime whitelist.
//! Does not own: mutation policy, endpoint authorization, config seeding, or DTO projection.
//! Boundary: ops converts complete model state to and from this memory-ID-61 record.

use crate::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, memory::VirtualMemory,
    },
    model::runtime_whitelist::MAX_RUNTIME_WHITELIST_RECORD_BYTES,
    role_contract::allocation::memory::runtime_whitelist::RUNTIME_WHITELIST_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

const RUNTIME_WHITELIST_RECORD_KEY: u8 = 0;

eager_static! {
    static RUNTIME_WHITELIST: RefCell<
        StableBtreeMap<u8, RuntimeWhitelistRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.runtime.whitelist.v1",
        ty = RuntimeWhitelistStore,
        id = RUNTIME_WHITELIST_ID,
    )));
}

/// Stable semantic outcome of one accepted mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeWhitelistMutationOutcomeRecord {
    Added,
    AlreadyPresent,
    Removed,
    AlreadyAbsent,
}

/// Exact retained response for one accepted operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeWhitelistMutationResponseRecord {
    pub outcome: RuntimeWhitelistMutationOutcomeRecord,
    pub principal: Principal,
    pub revision: u64,
    pub membership_digest: [u8; 32],
}

/// Sole retained exact-retry record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeWhitelistOperationRecord {
    pub operation_id: [u8; 32],
    pub request_hash: [u8; 32],
    pub result: RuntimeWhitelistMutationResponseRecord,
}

/// Canonical schema-1 runtime-whitelist state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeWhitelistRecord {
    pub schema_version: u32,
    pub principals: Vec<Principal>,
    pub revision: u64,
    pub membership_digest: [u8; 32],
    pub last_operation: Option<RuntimeWhitelistOperationRecord>,
}

impl RuntimeWhitelistRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RuntimeWhitelistRecord";
}

impl_storable_bounded!(
    RuntimeWhitelistRecord,
    MAX_RUNTIME_WHITELIST_RECORD_BYTES,
    false
);

/// Test/audit snapshot of the optional fresh-install record.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeWhitelistData {
    pub record: Option<RuntimeWhitelistRecord>,
}

impl RuntimeWhitelistData {
    pub const STATE_CONTRACT_NAME: &'static str = "RuntimeWhitelistData";
}

/// Single-record stable owner.
pub struct RuntimeWhitelistStore;

impl RuntimeWhitelistStore {
    #[must_use]
    pub(crate) fn get() -> Option<RuntimeWhitelistRecord> {
        RUNTIME_WHITELIST.with_borrow(|store| store.get(&RUNTIME_WHITELIST_RECORD_KEY))
    }

    pub(crate) fn initialize(record: RuntimeWhitelistRecord) -> bool {
        RUNTIME_WHITELIST.with_borrow_mut(|store| {
            if store.get(&RUNTIME_WHITELIST_RECORD_KEY).is_some() {
                return false;
            }
            store.insert(RUNTIME_WHITELIST_RECORD_KEY, record);
            true
        })
    }

    pub(crate) fn replace(record: RuntimeWhitelistRecord) -> bool {
        RUNTIME_WHITELIST.with_borrow_mut(|store| {
            if store.get(&RUNTIME_WHITELIST_RECORD_KEY).is_none() {
                return false;
            }
            store.insert(RUNTIME_WHITELIST_RECORD_KEY, record);
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_current_record_fits_the_frozen_bound() {
        let principals = (0..256).map(principal).collect::<Vec<_>>();
        let record = RuntimeWhitelistRecord {
            schema_version: 1,
            principals: principals.clone(),
            revision: u64::MAX,
            membership_digest: [0xff; 32],
            last_operation: Some(RuntimeWhitelistOperationRecord {
                operation_id: [0xfe; 32],
                request_hash: [0xfd; 32],
                result: RuntimeWhitelistMutationResponseRecord {
                    outcome: RuntimeWhitelistMutationOutcomeRecord::AlreadyPresent,
                    principal: principals[255],
                    revision: u64::MAX,
                    membership_digest: [0xfc; 32],
                },
            }),
        };
        let stable_bytes = crate::cdk::serialize::serialize(&record).expect("record CBOR");
        assert_eq!(stable_bytes.len(), 8_417);
        assert!(stable_bytes.len() <= MAX_RUNTIME_WHITELIST_RECORD_BYTES as usize);
    }

    fn principal(index: usize) -> Principal {
        let mut bytes = [0_u8; 29];
        bytes[..8].copy_from_slice(
            &u64::try_from(index)
                .expect("fixture index fits u64")
                .to_be_bytes(),
        );
        bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
        Principal::from_slice(&bytes)
    }
}
