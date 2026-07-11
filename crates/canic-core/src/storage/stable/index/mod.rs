pub mod app;
pub mod subnet;

use crate::{cdk::types::Principal, ids::CanisterRole};

///
/// IndexEntryRecord
///
/// One logical stable index row.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexEntryRecord {
    pub role: CanisterRole,
    pub pid: Principal,
}

impl IndexEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IndexEntryRecord";
}
