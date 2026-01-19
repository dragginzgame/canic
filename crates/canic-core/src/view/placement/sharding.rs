use crate::{cdk::candid::Principal, ids::CanisterRole};

///
/// ShardPlacement
///

#[derive(Clone, Debug)]
pub struct ShardPlacement {
    pub pool: String,
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub role: CanisterRole,
    pub created_at: u64,
}

impl ShardPlacement {
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;
}

///
/// ShardTenantAssignment
///

#[derive(Clone, Debug)]
pub struct ShardTenantAssignment {
    pub tenant: String,
    pub pid: Principal,
}
