use crate::{
    Error, Log, interface::request::create_canister_request, log, memory::CanisterDirectory,
    state::canister::CanisterCatalog,
};
use candid::Principal;

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    // Top-up pass
    for (ty, canister) in CanisterCatalog::export() {
        let Some(auto_create) = canister.attributes.auto_create else {
            continue;
        };

        for _ in 0..auto_create {
            create_canister_request::<()>(&ty.clone(), None).await?;
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
