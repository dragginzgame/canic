use crate::{
    Error,
    config::Config,
    memory::subnet::SubnetRegistry,
    ops::{
        prelude::*,
        request::{CreateCanisterParent, create_canister_request},
    },
};

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    let cfg = Config::try_get()?;

    // Top-up pass
    for (ty, canister) in &cfg.canisters {
        if canister.auto_create {
            create_canister_request::<()>(ty, CreateCanisterParent::Root, None).await?;
        }
    }

    // Report pass
    for canister in SubnetRegistry::all() {
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
