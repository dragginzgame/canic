use crate::{
    Error, Log, interface::request::create_canister_request, log, memory::SubnetDirectory,
    state::canister::CanisterRegistry,
};
use candid::Principal;

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    // Top-up pass
    for (ty, canister) in CanisterRegistry::export() {
        let Some(auto_create) = canister.attributes.auto_create else {
            continue;
        };

        for _ in 0..auto_create {
            create_canister_request::<()>(&ty.clone(), None).await?;
        }
    }

    // Report pass
    for (ty, entry) in SubnetDirectory::export() {
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
