use crate::{
    Error, Log, interface::request::create_canister_request, log, memory::SubnetDirectory,
    state::canister::CanisterRegistry,
};
use candid::Principal;

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    // Top-up pass
    for (kind, canister) in CanisterRegistry::export() {
        let Some(auto_create) = canister.attributes.auto_create else {
            continue;
        };

        for _ in 0..auto_create {
            // let this bubble up instead of unwrap()
            create_canister_request::<()>(&kind, None).await?;
        }
    }

    // Report pass
    for (kind, entry) in SubnetDirectory::export() {
        let canisters = entry
            .canisters
            .iter()
            .map(Principal::to_text)
            .collect::<Vec<_>>()
            .join(", ");

        log!(Log::Info, "🥫 {kind}: {canisters}");
    }

    Ok(())
}
