//! CanisterChildren
//!
//! Stable-memory–backed projection of direct child canisters for the
//! current canister.
//!
//! This is not a subnet-wide Registry. It is the per-canister direct-child
//! evidence consumed by local topology and placement workflows. Entries are
//! populated only through validated topology cascade application. Fleet Subnet
//! Root Component membership remains a separate control-plane authority.
//!
//! The contents are replaced wholesale on import.

use crate::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, memory::RuntimeMemory,
    },
    storage::{canister::CanisterRecord, prelude::*},
};
use std::cell::RefCell;

std::thread_local! {
    //
    // CANISTER_CHILDREN
    //
    static CANISTER_CHILDREN: RefCell<
        StableBtreeMap<Principal, CanisterChildRecord, RuntimeMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.runtime.canister_children.v1")),
    );
}

///
/// CanisterChildrenData
///
/// Canonical direct-child projection snapshot.
///

#[derive(Clone, Debug)]
pub struct CanisterChildrenData {
    pub entries: Vec<CanisterChildEntryRecord>,
}

/// Direct-child authority; infrastructure topology has no Component allocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterChildRecord {
    pub canister: CanisterRecord,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    pub allocation_operation_id: Option<[u8; 32]>,
}

impl_storable_bounded!(CanisterChildRecord, 384, false);

/// Canonical child snapshot row, including the allocation that owns the principal.
#[derive(Clone, Debug)]
pub struct CanisterChildEntryRecord {
    pub pid: Principal,
    pub record: CanisterChildRecord,
}

impl CanisterChildEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterChildEntryRecord";
}

impl CanisterChildrenData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterChildrenData";
}

///
/// CanisterChildren
///

pub struct CanisterChildren;

impl CanisterChildren {
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterChildRecord> {
        CANISTER_CHILDREN.with_borrow(|map| map.get(&pid))
    }

    #[must_use]
    pub fn export() -> CanisterChildrenData {
        CanisterChildrenData {
            entries: CANISTER_CHILDREN.with_borrow(|map| {
                map.iter()
                    .map(|entry| CanisterChildEntryRecord {
                        pid: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: CanisterChildrenData) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.pid, entry.record);
            }
        });
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::{
            serialize::{deserialize, serialize},
            structures::Storable,
        },
        config::schema::NAME_MAX_BYTES,
    };
    use ciborium::value::Value;

    #[test]
    fn child_allocation_field_is_bounded_and_required_even_when_null() {
        for allocation_operation_id in [None, Some([u8::MAX; 32])] {
            let record = CanisterChildRecord {
                canister: CanisterRecord {
                    role: CanisterRole::owned("x".repeat(NAME_MAX_BYTES)),
                    parent_pid: Some(Principal::from_slice(&[u8::MAX; 29])),
                    module_hash: Some(vec![u8::MAX; 32]),
                    created_at: u64::MAX,
                },
                allocation_operation_id,
            };
            let bytes = record.to_bytes();
            let crate::cdk::structures::storable::Bound::Bounded { max_size, .. } =
                CanisterChildRecord::BOUND
            else {
                panic!("direct-child records have a declared bound");
            };
            assert!(bytes.len() <= max_size as usize);
            assert_eq!(
                CanisterChildRecord::from_bytes(bytes.clone()).allocation_operation_id,
                allocation_operation_id,
            );
            let Value::Map(mut fields) = deserialize(&bytes).unwrap() else {
                panic!("child record encodes as a map");
            };
            let key = Value::Text("allocation_operation_id".into());
            let (_, value) = fields.iter().find(|(field, _)| field == &key).unwrap();
            if allocation_operation_id.is_none() {
                assert_eq!(value, &Value::Null);
            }
            fields.retain(|(field, _)| field != &key);
            let missing = serialize(&Value::Map(fields)).unwrap();
            assert!(deserialize::<CanisterChildRecord>(&missing).is_err());
        }
    }
}
