//! Module: ops::topology::input::mapper
//!
//! Responsibility: convert topology records into policy input views.
//! Does not own: topology policy, storage mutation, or endpoint DTO schemas.
//! Boundary: ops mapper used by topology workflows.

use crate::{
    model::topology::{TopologyEntry, TopologyRegistry},
    storage::stable::registry::subnet::SubnetRegistryData,
};

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
                .map(|entry| TopologyEntry {
                    pid: entry.pid,
                    role: entry.record.role,
                    parent_pid: entry.record.parent_pid,
                    module_hash: entry.record.module_hash,
                })
                .collect(),
        }
    }
}
