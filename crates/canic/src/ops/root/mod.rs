pub mod reserve;

use crate::{
    Error,
    interface::ic::get_current_subnet_pid,
    memory::{Env, topology::SubnetCanisterRegistry},
    ops::{
        context::cfg_current_subnet,
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
    let subnet_cfg = cfg_current_subnet()?;
    for ty in &subnet_cfg.auto_create {
        create_canister_request::<()>(ty, CreateCanisterParent::Root, None).await?;
    }

    // Report pass
    for canister in SubnetCanisterRegistry::all() {
        log!(Log::Info, "🥫 {} ({})", canister.ty, canister.pid);
    }

    Ok(())
}
