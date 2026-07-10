//! Module: role_contract
//!
//! Responsibility: own role capability, feature, allocation, and memory-ID policy.
//! Does not own: Cargo evidence collection, state records, descriptors, or rendering.
//! Boundary: host/build tooling supplies typed evidence and consumes pure resolution.

pub const CANONICAL_BUILD_MARKER_ENV: &str = "CANIC_ROLE_CONTRACT_VALIDATED";
pub const CANONICAL_BUILD_MARKER_VALUE: &str = "1";

pub mod allocation;
pub mod catalog;
mod model;
mod policy;
#[cfg(test)]
mod tests;

pub use model::{
    AllocationDefinition, AllocationOwner, BuiltInRoleKind, CanicFeatureEffect, CanicFeatureKey,
    MemoryId, ResolvedRoleContract, ResolvedStateAllocation, RoleCapabilityKey,
    RoleContractFinding, RoleContractInput, RoleContractResolution, RoleContractSource,
    RoleFeatureRequirement, SelectionProvenance, StateAllocationKey,
};
pub use policy::{
    built_in_role_capabilities, derive_role_capabilities, required_features_for_role,
    resolve_effective_features, resolve_role_contract,
};
