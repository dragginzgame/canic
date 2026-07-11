//! Module: ops::topology::input::mapper
//!
//! Responsibility: convert topology records into policy input views.
//! Does not own: topology policy, storage mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by topology workflows.

use crate::{
    cdk::types::Principal,
    domain::policy::pure::topology::{RegistryPolicyInput, TopologyPolicyInput},
    storage::{canister::CanisterRecord, stable::registry::subnet::SubnetRegistryData},
};

///
/// TopologyPolicyInputMapper
///
/// Operations-layer mapper for canister records and topology policy inputs.
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
/// Operations-layer mapper for subnet registry snapshots and policy inputs.
///

pub struct RegistryPolicyInputMapper;

impl RegistryPolicyInputMapper {
    #[must_use]
    pub fn data_to_policy_input(data: SubnetRegistryData) -> RegistryPolicyInput {
        RegistryPolicyInput {
            entries: data
                .entries
                .into_iter()
                .map(|entry| {
                    TopologyPolicyInputMapper::record_to_policy_input(entry.pid, entry.record)
                })
                .collect(),
        }
    }
}
