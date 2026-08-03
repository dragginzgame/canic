//! Module: model::topology
//!
//! Responsibility: own authoritative topology and index observation values.
//! Does not own: topology policy, stable record conversion, or storage access.

use crate::{domain::value::Principal, ids::CanisterRole};

/// One observed canister entry in the subnet topology.
pub struct TopologyEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    #[cfg(test)]
    pub parent_pid: Option<Principal>,
}

/// Complete observed subnet topology used for invariant evaluation.
pub struct TopologyRegistry {
    pub entries: Vec<TopologyEntry>,
}
