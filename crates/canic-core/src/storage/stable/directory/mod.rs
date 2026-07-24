pub mod fleet;
pub mod subnet;

use crate::{cdk::types::Principal, ids::CanisterRole};

///
/// DirectoryEntryRecord
///
/// One logical stable Directory row.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntryRecord {
    pub role: CanisterRole,
    pub pid: Principal,
}

impl DirectoryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "DirectoryEntryRecord";
}
