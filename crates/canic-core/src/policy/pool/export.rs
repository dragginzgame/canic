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
    policy::pool::PoolPolicyError,
};

///
/// Policy: may this pool entry be exported?
///
/// On success, returns the exact data required to perform the export.
/// Ops/workflow should treat this as authoritative.
///
/// Invariants enforced:
/// - status must be Ready
/// - role must be present
/// - module_hash must be present
///
pub fn can_export(entry: &CanisterPoolEntry) -> Result<(CanisterRole, Vec<u8>), PoolPolicyError> {
    match entry.status {
        CanisterPoolStatus::Ready => {}
        _ => return Err(PoolPolicyError::NotReadyForExport),
    }

    let role = entry.role.clone().ok_or(PoolPolicyError::MissingRole)?;

    let module_hash = entry
        .module_hash
        .clone()
        .ok_or(PoolPolicyError::MissingModuleHash)?;

    Ok((role, module_hash))
}
