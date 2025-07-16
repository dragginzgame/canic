use crate::{
    Error, interface::request::canister_create_request, memory::SubnetIndex,
    state::CanisterRegistry,
};
use candid::Principal;

// create_canisters
pub async fn create_canisters(controllers: &[Principal]) -> Result<(), Error> {
    // iterate canisters
    for (kind, data) in CanisterRegistry::get_data() {
        if data.attributes.auto_create && SubnetIndex::get_canister(&kind).is_none() {
            canister_create_request::<()>(&kind, controllers, None)
                .await
                .unwrap();
        }
    }

    Ok(())
}
