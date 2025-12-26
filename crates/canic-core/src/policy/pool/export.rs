// crates/<your-crate>/src/policy/pool/export.rs
//
// Pool export policy:
// - answers “may this pool entry be exported?”
// - extracts the data required for export
// - side-effect free
// - no storage mutation, no logging

use crate::{
    ids::CanisterRole,
    model::memory::pool::{CanisterPoolEntry, CanisterPoolStatus},
};

/// PoolExportPolicyError
/// Reasons why a pool entry may not be exported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolExportPolicyError {
    /// Pool entry is not in Ready state.
    NotReady,

    /// Pool entry is missing a role (type).
    MissingRole,

    /// Pool entry is missing a module hash.
    MissingModuleHash,
}

impl core::fmt::Display for PoolExportPolicyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotReady => write!(f, "pool entry is not ready for export"),
            Self::MissingRole => write!(f, "pool entry is missing role metadata"),
            Self::MissingModuleHash => write!(f, "pool entry is missing module hash"),
        }
    }
}

/// Policy: may this pool entry be exported?
///
/// On success, returns the exact data required to perform the export.
/// Ops/workflow should treat this as authoritative.
///
/// Invariants enforced:
/// - status must be Ready
/// - role must be present
/// - module_hash must be present
pub fn can_export(
    entry: &CanisterPoolEntry,
) -> Result<(CanisterRole, Vec<u8>), PoolExportPolicyError> {
    match entry.status {
        CanisterPoolStatus::Ready => {}
        _ => return Err(PoolExportPolicyError::NotReady),
    }

    let role = entry
        .role
        .clone()
        .ok_or(PoolExportPolicyError::MissingRole)?;

    let module_hash = entry
        .module_hash
        .clone()
        .ok_or(PoolExportPolicyError::MissingModuleHash)?;

    Ok((role, module_hash))
}
