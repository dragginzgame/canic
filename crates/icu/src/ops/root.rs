use crate::{
    Error,
    config::Config,
    memory::CanisterRegistry,
    ops::{prelude::*, request::create_canister_request},
};

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    let cfg = Config::try_get()?;

    // Top-up pass
    for (ty, canister) in &cfg.canisters {
        let Some(auto_create) = canister.auto_create else {
            continue;
        };

        for _ in 0..auto_create {
            create_canister_request::<()>(ty, None).await?;
        }
    }

    // Report pass
    for (pid, entry) in CanisterRegistry::export() {
        log!(
            Log::Info,
            "🥫 {} ({}) [{}]",
            entry.canister_type,
            pid,
            entry.status,
        );
    }

    Ok(())
}
