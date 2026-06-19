//! Module: ops::topology::input::mapper
//!
//! Responsibility: convert topology records into policy input views.
//! Does not own: topology policy, storage mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by topology workflows.

use crate::{
    cdk::types::Principal,
    storage::{canister::CanisterRecord, stable::registry::subnet::SubnetRegistryRecord},
    view::topology::{RegistryPolicyInput, TopologyPolicyInput},
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
