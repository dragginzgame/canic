use crate::{cdk::candid::Principal, ids::CanisterRole};

///
/// ShardPlacementView
///

#[derive(Clone, Debug)]
pub struct ShardPlacementView {
    pub pool: String,
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub role: CanisterRole,
    pub created_at: u64,
}

impl ShardPlacementView {
    pub const UNASSIGNED_SLOT: u32 = u32::MAX;
}

///
/// ShardTenantAssignmentView
///

#[derive(Clone, Debug)]
pub struct ShardTenantAssignmentView {
    pub tenant: String,
    pub pid: Principal,
}
