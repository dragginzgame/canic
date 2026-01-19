use crate::{
    cdk::types::Principal,
    domain::policy::topology::input::{RegistryPolicyInput, TopologyPolicyInput},
    storage::{canister::CanisterRecord, stable::registry::subnet::SubnetRegistryRecord},
};

///
/// TopologyPolicyInputMapper
///

pub struct TopologyPolicyInputMapper;

impl TopologyPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(pid: Principal, record: CanisterRecord) -> TopologyPolicyInput {
        TopologyPolicyInput {
            pid,
            role: record.role,
            parent_pid: record.parent_pid,
            module_hash: record.module_hash,
        }
    }
}

///
/// RegistryPolicyInputMapper
///

pub struct RegistryPolicyInputMapper;

impl RegistryPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(record: SubnetRegistryRecord) -> RegistryPolicyInput {
        RegistryPolicyInput {
            entries: record
                .entries
                .into_iter()
                .map(|(pid, entry)| TopologyPolicyInputMapper::record_to_policy_input(pid, entry))
                .collect(),
        }
    }
}
