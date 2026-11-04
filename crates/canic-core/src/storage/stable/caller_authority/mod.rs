//! Module: storage::stable::caller_authority
//!
//! Stable receiver header and independently keyed source, fence and original-operation rows.
//!
//! Ops validates transitions and capacity before committing these protected records.

use crate::{
    cdk::structures::{DefaultMemoryImpl, StableBTreeMap as BTreeMap, StableCell},
    model::caller_authority::{
        CallerReceiptRecord, CallerReceiverRecord, CallerRowKey, CallerSourceRecord,
    },
    storage::prelude::*,
};
use canic_contracts::ids::CallerComponentInstallation;
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
    static HEADER: RefCell<StableCell<Option<CallerReceiverRecord>, DefaultMemoryImpl>> =
        RefCell::new({
            let allocation = crate::memory::open(crate::memory::keys::CALLER_AUTHORITY_HEADER_V1);
            if ic_storage::Memory::size(&allocation) == 0 {
                StableCell::create(allocation, None).expect("create admitted stable cell")
            } else {
                StableCell::load(allocation, canic_contracts::serialization::STORAGE_LIMITS.bytes).expect("retained stable cell")
            }
        });
    static ROWS: RefCell<BTreeMap<CallerRowKey, CallerRowRecord, DefaultMemoryImpl>> =
        RefCell::new({
            let allocation = crate::memory::open(crate::memory::keys::CALLER_AUTHORITY_ROWS_V1);
            if ic_storage::Memory::size(&allocation) == 0 {
                BTreeMap::create(allocation).expect("open admitted stable collection")
            } else {
                BTreeMap::load(allocation).expect("open admitted stable collection")
            }
        });
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
            header.set(data.header).expect("commit stable fixture");
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
            header
                .set(Some(record))
                .expect("commit caller authority header");
        });
    }

    #[cfg(test)]
    pub(crate) fn reset() {
        HEADER.with_borrow_mut(|header| {
            header.set(None).expect("commit stable fixture");
        });
        for (key, _) in CallerRowStore::rows() {
            ROWS.with_borrow_mut(|rows| {
                rows.remove(&key).expect("Storage operation failed");
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
                    ic_platform::principal::ExternalPrincipal::management_canister(),
                ))
            },
            Excluded,
        );
        ROWS.with_borrow(|rows| {
            rows.range((start, Unbounded))
                .map(|entry| entry.expect("Storage iteration failed"))
                .take(limit)
                .map_while(|entry| {
                    let (key, row) = entry.into_pair().expect("decode Storage entry");
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
            rows.remove(key).expect("Storage operation failed");
        });
    }

    pub(crate) fn get(key: &CallerRowKey) -> Option<CallerRowRecord> {
        ROWS.with_borrow(|rows| rows.get(key).expect("Storage operation failed"))
    }
    pub(crate) fn insert(key: CallerRowKey, record: CallerRowRecord) {
        ROWS.with_borrow_mut(|rows| {
            rows.insert(key, record).expect("Storage operation failed");
        });
    }
    pub(crate) fn rows() -> Vec<(CallerRowKey, CallerRowRecord)> {
        ROWS.with_borrow(|rows| {
            let mut projection = Vec::new();
            for entry in rows
                .iter()
                .map(|entry| entry.expect("Storage iteration failed"))
            {
                projection.push(entry.into_pair().expect("decode Storage entry"));
            }
            projection
        })
    }
}
