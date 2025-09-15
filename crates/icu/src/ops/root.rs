use crate::{
    Error,
    config::Config,
    memory::CanisterDirectory,
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
    let export = CanisterDirectory::export();
    for (ty, entry) in export.entries {
        let canisters = entry
            .canisters
            .iter()
            .map(Principal::to_text)
            .collect::<Vec<_>>()
            .join(", ");

        log!(Log::Info, "🥫 {ty}: {canisters}");
    }

    Ok(())
}
