// Pool authority policy:
// - answers “is the caller authorized to perform pool admin operations?”
// - side-effect free
// - does not log / mutate / schedule

use super::PoolPolicyError;

/// Require that the caller is authorized to perform pool admin operations.
///
/// Current policy: root-only.
pub const fn require_pool_admin(is_root: bool) -> Result<(), PoolPolicyError> {
    if is_root {
        Ok(())
    } else {
        Err(PoolPolicyError::NotAuthorized)
    }
}
