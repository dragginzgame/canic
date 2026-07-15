//! Module: ops::topology::input::mapper
//!
//! Responsibility: convert topology records into policy input views.
//! Does not own: topology policy, storage mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by topology workflows.

use crate::{
    cdk::types::Principal,
    model::topology::{TopologyEntry, TopologyRegistry},
    storage::{canister::CanisterRecord, stable::registry::subnet::SubnetRegistryData},
};

///
/// TopologyEntryMapper
///
/// Operations-layer mapper for canister records and topology policy inputs.
///

pub struct TopologyEntryMapper;

impl TopologyEntryMapper {
    #[must_use]
    pub fn record_to_entry(pid: Principal, record: CanisterRecord) -> TopologyEntry {
        TopologyEntry {
            pid,
            role: record.role,
            parent_pid: record.parent_pid,
            module_hash: record.module_hash,
        }
    }
}

///
/// TopologyRegistryMapper
///
/// Operations-layer mapper for subnet registry snapshots and policy inputs.
///

pub struct TopologyRegistryMapper;

impl TopologyRegistryMapper {
    #[must_use]
    pub fn data_to_registry(data: SubnetRegistryData) -> TopologyRegistry {
        TopologyRegistry {
            entries: data
                .entries
                .into_iter()
                .map(|entry| TopologyEntryMapper::record_to_entry(entry.pid, entry.record))
                .collect(),
        }
    }
}
