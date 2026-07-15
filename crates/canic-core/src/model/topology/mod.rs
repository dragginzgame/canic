//! Module: model::topology
//!
//! Responsibility: own authoritative topology and index observation values.
//! Does not own: topology policy, stable record conversion, or storage access.

use crate::{domain::value::Principal, ids::CanisterRole};

/// One observed canister entry in the subnet topology.
pub struct TopologyEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

/// Complete observed subnet topology used for invariant evaluation.
pub struct TopologyRegistry {
    pub entries: Vec<TopologyEntry>,
}

/// One observed role binding in an app or subnet index.
pub struct TopologyIndexEntry {
    pub role: CanisterRole,
    pub pid: Principal,
}
