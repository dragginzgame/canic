// Pool authority policy:
// - answers “is the caller authorized to perform pool admin operations?”
// - side-effect free
// - does not log / mutate / schedule

use super::PoolPolicyError;
use crate::ops::OpsError;

/// Require that the caller is authorized to perform pool admin operations.
///
/// Current policy: root-only (delegates to existing ops root check).
///
/// Notes:
/// - Keeping this wrapper in policy allows future changes (multi-admin,
///   governance, capability-based auth) without touching ops/workflows.
/// - This function is intentionally non-async.
pub fn require_pool_admin() -> Result<(), PoolPolicyError> {
    OpsError::require_root().map_err(|_| PoolPolicyError::NotAuthorized)
}
