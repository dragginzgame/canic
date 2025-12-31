use crate::{
    cdk::types::Principal,
    ops::{
        ic::{Network, build_network, canister_status_internal},
        storage::registry::SubnetRegistryOps,
    },
    policy::pool::{
        PoolPolicyError,
        admissibility::{policy_can_enter_pool, policy_is_importable_on_local},
    },
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
async fn probe_importable_on_local(pid: Principal) -> Result<(), String> {
    #[cfg(test)]
    if let Some(override_value) = TEST_IMPORTABLE_OVERRIDE.with(|slot| *slot.borrow()) {
        return if override_value {
            Ok(())
        } else {
            Err("test override: non-importable".to_string())
        };
    }

    if !is_local_build() {
        return Ok(());
    }

    match canister_status_internal(pid).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

/// Policy: may this canister *enter or remain* in the pool?
///
/// This is the main workflow entrypoint ops/workflows should use.
pub async fn check_can_enter_pool(pid: Principal) -> Result<(), PoolPolicyError> {
    let registered_in_subnet = SubnetRegistryOps::get(pid).is_some();
    let importable_on_local = probe_importable_on_local(pid).await;

    policy_can_enter_pool(pid, registered_in_subnet, importable_on_local)
}

/// Convenience helper when you only want the local-routability decision (no registry check).
pub async fn check_importable_on_local(pid: Principal) -> Result<(), PoolPolicyError> {
    let importable_on_local = probe_importable_on_local(pid).await;

    policy_is_importable_on_local(pid, importable_on_local)
}
