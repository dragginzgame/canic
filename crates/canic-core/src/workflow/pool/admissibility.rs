use crate::{
    domain::policy::pool::{PoolPolicyError, admissibility::policy_can_enter_pool},
    ids::BuildNetwork,
    ops::{
        ic::{mgmt::MgmtOps, network::NetworkOps},
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::prelude::*,
};

#[inline]
fn is_local_build() -> bool {
    NetworkOps::build_network() == Some(BuildNetwork::Local)
}

/// Returns Ok(()) iff the canister is routable in the current local replica.
///
/// Local-only precondition check.
/// Must be cheap, non-destructive, and side-effect free.
async fn probe_importable_on_local(pid: Principal) -> Result<(), String> {
    if !is_local_build() {
        return Ok(());
    }

    match MgmtOps::canister_status(pid).await {
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
