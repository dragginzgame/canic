//! Root-specific orchestration flows.
//!
//! The root canister can pre-create canisters configured with `auto_create`
//! and emit a topology report. These helpers bundle that logic so the
//! lifecycle macros can trigger it during installation.

use crate::{
    Error,
    config::Config,
    memory::topology::SubnetTopology,
    ops::{
        prelude::*,
        request::{CreateCanisterParent, create_canister_request},
    },
};

/// Ensure all auto-create canisters exist and log the current topology.
pub async fn root_create_canisters() -> Result<(), Error> {
    let cfg = Config::try_get()?;

    // Top-up pass
    for (ty, canister) in &cfg.canisters {
        if canister.auto_create {
            create_canister_request::<()>(ty, CreateCanisterParent::Root, None).await?;
        }
    }

    // Report pass
    for canister in SubnetTopology::all() {
        log!(
            Log::Info,
            "🥫 {} ({}) [{}]",
            canister.ty,
            canister.pid,
            canister.status
        );
    }

    Ok(())
}
