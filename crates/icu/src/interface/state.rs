use crate::{
    Error, interface::request::canister_create_request, memory::SubnetIndex,
    state::CanisterRegistry,
};
use candid::Principal;

// create_canisters
pub async fn create_canisters(controllers: &[Principal]) -> Result<(), Error> {
    // iterate canisters
    for (kind, data) in CanisterRegistry::export() {
        if data.attributes.auto_create && SubnetIndex::get(&kind).is_none() {
            canister_create_request::<()>(&kind, controllers, None)
                .await
                .unwrap();
        }
    }

    Ok(())
}
