// Pool admissibility policy:
// - answers “may this PID enter / remain in the pool?”
// - side-effect free (no ops calls)
// - does NOT log, schedule, or mutate storage

use crate::{cdk::types::Principal, domain::policy::pool::PoolPolicyError};

/// Policy: may this canister *enter or remain* in the pool?
///
/// Callers must provide:
/// - whether the PID is still registered in the subnet registry
/// - the local importability check result (Ok/Err details)
pub fn policy_can_enter_pool(
    pid: Principal,
    registered_in_subnet: bool,
    importable_on_local: Result<(), String>,
) -> Result<(), PoolPolicyError> {
    if registered_in_subnet {
        return Err(PoolPolicyError::RegisteredInSubnet(pid));
    }

    policy_is_importable_on_local(pid, importable_on_local)
}

/// Convenience helper when you only want the local-routability decision (no registry check).
pub fn policy_is_importable_on_local(
    pid: Principal,
    importable_on_local: Result<(), String>,
) -> Result<(), PoolPolicyError> {
    importable_on_local.map_err(|details| PoolPolicyError::NonImportableOnLocal { pid, details })
}
