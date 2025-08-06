use crate::{
    Error, interface::request::canister_create_request, memory::SubnetIndex,
    state::CanisterRegistry,
};

// create_canisters
pub async fn create_canisters() -> Result<(), Error> {
    // iterate canisters
    for (kind, data) in CanisterRegistry::export() {
        if data.attributes.auto_create && SubnetIndex::get(&kind).is_none() {
            canister_create_request::<()>(&kind, None).await.unwrap();
        }
    }

    Ok(())
}
