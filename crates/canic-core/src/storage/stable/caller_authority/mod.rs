//! Module: storage::stable::caller_authority
//!
//! Stable receiver header and independently keyed source, fence and original-operation rows.
//!
//! Ops validates transitions and capacity before committing these protected records.

use crate::{
    cdk::{
        bounded_cell::BoundedCell,
        structures::{DefaultMemoryImpl, btreemap::BTreeMap, memory::RuntimeMemory},
    },
    ids::CallerComponentInstallation,
    model::caller_authority::{
        CallerReceiptRecord, CallerReceiverRecord, CallerRowKey, CallerSourceRecord,
    },
    role_contract::allocation::memory::caller_authority::{
        CALLER_AUTHORITY_HEADER_ID, CALLER_AUTHORITY_ROWS_ID,
    },
    storage::prelude::*,
};
use std::cell::RefCell;

/// Bound for one receipt or source, independent of receiver census size.
pub const MAX_CALLER_ROW_BYTES: u32 = 8_192;

///
/// CallerRowRecord
///
/// Independent row; receipt acknowledgements replace one row, never the receiver census.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerRowRecord {
    Source(CallerSourceRecord),
    ComponentFence(CallerComponentInstallation),
    Receipt(Box<CallerReceiptRecord>),
}

impl CallerRowRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CallerRowRecord";
}

impl CallerReceiverRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CallerReceiverRecord";
}

///
/// CallerAuthorityData
///
/// Canonical snapshot of the receiver header and its complete indexed rows.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerAuthorityData {
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    header: Option<CallerReceiverRecord>,
    rows: Vec<(CallerRowKey, CallerRowRecord)>,
}

impl CallerAuthorityData {
    pub const STATE_CONTRACT_NAME: &'static str = "CallerAuthorityData";
}

impl_storable_bounded!(CallerRowKey, 256, false);
impl_storable_bounded!(CallerRowRecord, MAX_CALLER_ROW_BYTES, false);
impl_storable_bounded!(CallerReceiverRecord, MAX_CALLER_ROW_BYTES, false);

thread_local! {
    static HEADER: RefCell<BoundedCell<Option<CallerReceiverRecord>, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(BoundedCell::init(crate::ic_memory_key!(
            authority = CANIC_CORE_MEMORY_AUTHORITY,
            key = "canic.core.caller_authority.header.v1",
            ty = CallerReceiverStore,
            id = CALLER_AUTHORITY_HEADER_ID,
        ), None));
    static ROWS: RefCell<BTreeMap<CallerRowKey, CallerRowRecord, RuntimeMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(crate::ic_memory_key!(
            authority = CANIC_CORE_MEMORY_AUTHORITY,
            key = "canic.core.caller_authority.rows.v1",
            ty = CallerRowStore,
            id = CALLER_AUTHORITY_ROWS_ID,
        )));
}

/// Receiver's durable control-header owner.
pub struct CallerReceiverStore;

/// Receiver's durable indexed-row owner.
pub struct CallerRowStore;

impl CallerReceiverStore {
    #[cfg(test)]
    pub(crate) fn export() -> CallerAuthorityData {
        CallerAuthorityData {
            header: Self::get(),
            rows: CallerRowStore::rows(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: CallerAuthorityData) {
        Self::reset();
        HEADER.with_borrow_mut(|header| {
            header.set(data.header);
        });
        for (key, row) in data.rows {
            CallerRowStore::insert(key, row);
        }
    }
    pub(crate) fn get() -> Option<CallerReceiverRecord> {
        HEADER.with_borrow(|header| header.get().clone())
    }
    pub(crate) fn set(record: CallerReceiverRecord) {
        HEADER.with_borrow_mut(|header| {
            header.set(Some(record));
        });
    }

    #[cfg(test)]
    pub(crate) fn reset() {
        HEADER.with_borrow_mut(|header| {
            header.set(None);
        });
        for (key, _) in CallerRowStore::rows() {
            ROWS.with_borrow_mut(|rows| {
                rows.remove(&key);
            });
        }
    }
}

impl CallerRowStore {
    /// Traverse only source rows with a bounded successor observation.
    pub(crate) fn source_page(
        after: Option<CallerRowKey>,
        limit: usize,
    ) -> Vec<(CallerRowKey, CallerSourceRecord)> {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        let start = after.map_or_else(
            || {
                Included(CallerRowKey::source(
                    candid::Principal::management_canister(),
                ))
            },
            Excluded,
        );
        ROWS.with_borrow(|rows| {
            rows.range((start, Unbounded))
                .take(limit)
                .map_while(|entry| {
                    let (key, row) = entry.into_pair();
                    match row {
                        CallerRowRecord::Source(source) => Some((key, source)),
                        _ => None,
                    }
                })
                .collect()
        })
    }

    pub(crate) fn remove(key: &CallerRowKey) {
        ROWS.with_borrow_mut(|rows| {
            rows.remove(key);
        });
    }

    pub(crate) fn get(key: &CallerRowKey) -> Option<CallerRowRecord> {
        ROWS.with_borrow(|rows| rows.get(key))
    }
    pub(crate) fn insert(key: CallerRowKey, record: CallerRowRecord) {
        ROWS.with_borrow_mut(|rows| {
            rows.insert(key, record);
        });
    }
    pub(crate) fn rows() -> Vec<(CallerRowKey, CallerRowRecord)> {
        ROWS.with_borrow(|rows| {
            let mut projection = Vec::new();
            for entry in rows.iter() {
                projection.push(entry.into_pair());
            }
            projection
        })
    }
}
