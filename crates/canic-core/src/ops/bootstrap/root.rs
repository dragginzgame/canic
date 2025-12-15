use crate::{
    Error,
    cdk::api::canister_self,
    log::Topic,
    ops::{
        command::request::{CreateCanisterParent, create_canister_request},
        config::ConfigOps,
        ic::get_current_subnet_pid,
        prelude::*,
        storage::{env::EnvOps, topology::SubnetCanisterRegistryOps},
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
