pub mod response;

use crate::{
    Error, Log, interface::request::create_canister_request, log, memory::SubnetIndex,
    state::CanisterRegistry,
};

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    for (kind, data) in CanisterRegistry::export() {
        if data.attributes.auto_create && SubnetIndex::get(&kind).is_none() {
            create_canister_request::<()>(&kind, None).await.unwrap();
        }
    }

    log!(Log::Info, "🌐 [SUBNET INDEX]");
    for (kind, pid) in SubnetIndex::export() {
        log!(Log::Info, "{kind}: {pid}");
    }

    Ok(())
}
