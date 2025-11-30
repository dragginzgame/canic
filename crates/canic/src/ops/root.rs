use crate::{
    Error,
    interface::ic::get_current_subnet_pid,
    log::Topic,
    model::memory::{Env, topology::SubnetCanisterRegistry},
    ops::{
        config::ConfigOps,
        prelude::*,
        request::{CreateCanisterParent, create_canister_request},
    },
};

pub async fn root_set_subnet_id() {
    // set subnet_id asynchrously, but before its needed
    if let Ok(Some(subnet_pid)) = get_current_subnet_pid().await {
        Env::set_subnet_pid(subnet_pid);
    }
}

/// Ensure all auto-create canisters exist and log the current topology.
pub async fn root_create_canisters() -> Result<(), Error> {
    // Top-up pass
    let subnet_cfg = ConfigOps::current_subnet()?;
    for ty in &subnet_cfg.auto_create {
        create_canister_request::<()>(ty, CreateCanisterParent::Root, None).await?;
    }

    // Report pass
    for canister in SubnetCanisterRegistry::export() {
        log!(Topic::Init, Info, "🥫 {} ({})", canister.ty, canister.pid);
    }

    Ok(())
}
