use crate::{
    cdk::candid::Nat,
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    dto::icp_refill::{IcpRefillErrorCode, IcpRefillStatus},
    eager_static, impl_storable_bounded,
    storage::{prelude::*, stable::memory::observability::ICP_REFILL_RECORDS_ID},
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::cell::RefCell;

eager_static! {
    //
    // ICP_REFILL_RECORDS
    //
    static ICP_REFILL_RECORDS: RefCell<IcpRefillRecords> =
        RefCell::new(IcpRefillRecords::new(StableBtreeMap::init(
            crate::ic_memory_key!("canic.core.icp_refill_records.v1", IcpRefillRecords, ICP_REFILL_RECORDS_ID),
        )));
}

///
/// IcpRefillRecordKey
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IcpRefillRecordKey(pub u64);

impl_storable_bounded!(IcpRefillRecordKey, 16, false);

///
/// IcpRefillRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcpRefillRecord {
    pub id: u64,
    pub operation_id: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<[u8; 32]>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub cmc_to_account_owner: Principal,
    pub cmc_to_account_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub ledger_block_index: Option<u64>,
    pub notify_attempts: u32,
    pub cycles_sent: Option<Nat>,
    pub status: IcpRefillStatus,
    pub error_code: Option<IcpRefillErrorCode>,
    pub error_message: Option<String>,
    pub refund_block_index: Option<u64>,
    pub transaction_too_old_min_block_index: Option<u64>,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
}

impl IcpRefillRecord {
    pub const STORABLE_MAX_SIZE: u32 = 4096;
}

impl_storable_bounded!(IcpRefillRecord, IcpRefillRecord::STORABLE_MAX_SIZE, false);

///
/// IcpRefillRecords
///

pub struct IcpRefillRecords {
    map: StableBtreeMap<IcpRefillRecordKey, IcpRefillRecord, VirtualMemory<DefaultMemoryImpl>>,
}

impl IcpRefillRecords {
    pub const fn new(
        map: StableBtreeMap<IcpRefillRecordKey, IcpRefillRecord, VirtualMemory<DefaultMemoryImpl>>,
    ) -> Self {
        Self { map }
    }

    pub(crate) fn insert(record: IcpRefillRecord) -> Option<IcpRefillRecord> {
        ICP_REFILL_RECORDS
            .with_borrow_mut(|records| records.map.insert(IcpRefillRecordKey(record.id), record))
    }

    #[must_use]
    pub(crate) fn get(id: u64) -> Option<IcpRefillRecord> {
        ICP_REFILL_RECORDS.with_borrow(|records| records.map.get(&IcpRefillRecordKey(id)))
    }

    #[must_use]
    pub(crate) fn entries(
        offset: usize,
        limit: usize,
    ) -> Vec<(IcpRefillRecordKey, IcpRefillRecord)> {
        ICP_REFILL_RECORDS.with_borrow(|records| {
            records
                .map
                .iter()
                .skip(offset)
                .take(limit)
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }
}
