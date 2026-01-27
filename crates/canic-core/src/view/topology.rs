use crate::{cdk::types::Principal, ids::CanisterRole};

///
/// TopologyPolicyInput
///

pub struct TopologyPolicyInput {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

///
/// RegistryPolicyInput
///

pub struct RegistryPolicyInput {
    pub entries: Vec<TopologyPolicyInput>,
}
