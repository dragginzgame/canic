use crate::{
    Error, Log, config::Config, interface::request::create_canister_request, log,
    memory::CanisterDirectory,
};
use candid::Principal;

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
    for (ty, entry) in CanisterDirectory::export() {
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
