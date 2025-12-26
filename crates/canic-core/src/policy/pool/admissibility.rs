// Pool admissibility policy:
// - answers “may this PID enter / remain in the pool?”
// - side-effect free (may perform read-only mgmt/status checks on local)
// - does NOT log, schedule, or mutate storage

use crate::{
    cdk::types::Principal,
    ops::{
        ic::{Network, build_network, canister_status},
        storage::topology::SubnetCanisterRegistryOps,
    },
    policy::pool::PoolPolicyError,
};

#[inline]
fn is_local_build() -> bool {
    build_network() == Some(Network::Local)
}

#[cfg(test)]
thread_local! {
    static TEST_IMPORTABLE_OVERRIDE: std::cell::RefCell<Option<bool>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn set_test_importable_override(value: Option<bool>) {
    TEST_IMPORTABLE_OVERRIDE.with(|slot| *slot.borrow_mut() = value);
}

/// Returns Ok(()) iff the canister is routable in the current local replica.
///
/// Local-only precondition check.
/// Must be cheap, non-destructive, and side-effect free.
async fn check_importable_on_local(pid: Principal) -> Result<(), PoolPolicyError> {
    #[cfg(test)]
    if let Some(override_value) = TEST_IMPORTABLE_OVERRIDE.with(|slot| *slot.borrow()) {
        return if override_value {
            Ok(())
        } else {
            Err(PoolPolicyError::NonImportableOnLocal {
                pid,
                details: "test override: non-importable".to_string(),
            })
        };
    }

    if !is_local_build() {
        return Ok(());
    }

    match canister_status(pid).await {
        Ok(_) => Ok(()),
        Err(err) => Err(PoolPolicyError::NonImportableOnLocal {
            pid,
            details: err.to_string(),
        }),
    }
}

/// Policy: may this canister *enter or remain* in the pool?
///
/// This is the main policy entrypoint ops/workflows should use.
///
/// Notes:
/// - On non-local networks: always admissible (subject to other policies).
/// - On local: must be importable/routable.
/// - Additionally: pool membership is blocked if the PID is still in the subnet registry.
pub async fn assert_can_import(pid: Principal) -> Result<(), PoolPolicyError> {
    if SubnetCanisterRegistryOps::get(pid).is_some() {
        return Err(PoolPolicyError::RegisteredInSubnet(pid));
    }

    check_importable_on_local(pid).await
}

/// Convenience helper when you only want the local-routability decision (no registry check).
pub async fn is_importable_on_local(pid: Principal) -> Result<(), PoolPolicyError> {
    check_importable_on_local(pid).await
}
