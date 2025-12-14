use crate::{
    Error,
    cdk::api::canister_self,
    interface::ic::get_current_subnet_pid,
    log::Topic,
    ops::{
        config::ConfigOps,
        model::memory::{EnvOps, topology::SubnetCanisterRegistryOps},
        prelude::*,
        request::{CreateCanisterParent, create_canister_request},
    },
};

pub async fn root_set_subnet_id() {
    // set subnet_id asynchrously, but before its needed
    if let Ok(Some(subnet_pid)) = get_current_subnet_pid().await {
        EnvOps::set_subnet_pid(subnet_pid);
        return;
    }

    // fallback for environments without the registry (e.g., PocketIC)
    let fallback = canister_self();
    EnvOps::set_subnet_pid(fallback);
    log!(
        Topic::Topology,
        Warn,
        "get_current_subnet_pid unavailable; using self as subnet: {fallback}"
    );
}

/// Ensure all auto-create canisters exist and log the current topology.
pub async fn root_create_canisters() -> Result<(), Error> {
    let subnet_cfg = ConfigOps::current_subnet()?;

    // create pass
    for ty in &subnet_cfg.auto_create {
        create_canister_request::<()>(ty, CreateCanisterParent::Root, None).await?;
    }

    // Report pass
    for canister in SubnetCanisterRegistryOps::export() {
        log!(Topic::Init, Info, "🥫 {} ({})", canister.role, canister.pid);
    }

    Ok(())
}
