// crates/<your-crate>/src/policy/pool/export.rs
//
// Pool export policy:
// - answers “may this pool entry be exported?”
// - extracts the data required for export
// - side-effect free
// - no storage mutation, no logging

use crate::{ids::CanisterRole, policy::pool::PoolPolicyError};

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
pub fn can_export(
    is_ready: bool,
    role: Option<CanisterRole>,
    module_hash: Option<Vec<u8>>,
) -> Result<(CanisterRole, Vec<u8>), PoolPolicyError> {
    if !is_ready {
        return Err(PoolPolicyError::NotReadyForExport);
    }

    let role = role.ok_or(PoolPolicyError::MissingRole)?;

    let module_hash = module_hash.ok_or(PoolPolicyError::MissingModuleHash)?;

    Ok((role, module_hash))
}
