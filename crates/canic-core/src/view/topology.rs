//! Module: view::topology
//!
//! Responsibility: define topology policy input projections.
//! Does not own: canister records, registry storage, or topology decisions.
//! Boundary: ops mappers produce these views for topology policy.

use crate::{cdk::types::Principal, ids::CanisterRole};

///
/// TopologyPolicyInput
///
/// Read-only projection of one canister for topology policy evaluation.
/// Owned by view and produced by topology ops mappers.
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
/// Read-only registry snapshot for topology policy evaluation.
/// Owned by view and produced by topology ops mappers.
///

pub struct RegistryPolicyInput {
    pub entries: Vec<TopologyPolicyInput>,
}
