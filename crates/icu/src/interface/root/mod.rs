pub mod response;

use crate::{
    Error, interface::request::create_canister_request, memory::SubnetIndex,
    state::CanisterRegistry,
};

// root_create_canisters
pub async fn root_create_canisters() -> Result<(), Error> {
    for (kind, data) in CanisterRegistry::export() {
        if data.attributes.auto_create && SubnetIndex::get(&kind).is_none() {
            create_canister_request::<()>(&kind, None).await.unwrap();
        }
    }

    Ok(())
}
