use crate::{
    cdk::types::Principal, dto::canister::CanisterInfo, storage::canister::CanisterRecord,
};

///
/// CanisterRecordMapper
///

pub struct CanisterRecordMapper;

impl CanisterRecordMapper {
    #[must_use]
    pub fn record_to_response(pid: Principal, record: CanisterRecord) -> CanisterInfo {
        CanisterInfo {
            pid,
            role: record.role,
            parent_pid: record.parent_pid,
            module_hash: record.module_hash,
            created_at: record.created_at,
        }
    }
}
