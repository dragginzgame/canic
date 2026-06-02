use crate::storage::stable::icp_refill::{IcpRefillRecord, IcpRefillRecordKey, IcpRefillRecords};

///
/// IcpRefillRecordOps
///

#[allow(dead_code)]
pub struct IcpRefillRecordOps;

impl IcpRefillRecordOps {
    #[allow(dead_code)]
    pub fn insert(record: IcpRefillRecord) -> Option<IcpRefillRecord> {
        IcpRefillRecords::insert(record)
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn get(id: u64) -> Option<IcpRefillRecord> {
        IcpRefillRecords::get(id)
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn entries() -> Vec<(IcpRefillRecordKey, IcpRefillRecord)> {
        IcpRefillRecords::entries(0, usize::MAX)
    }
}
