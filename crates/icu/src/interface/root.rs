use crate::{
    Error,
    config::Config,
    interface::{
        InterfaceError,
        cascade::subnet_index_cascade,
        ic::{IcError, canister_self, ic_create_canister},
    },
    memory::{CanisterState, ChildIndex, SubnetIndex, canister::CanisterParent},
    state::CanisterRegistry,
};
use candid::{Principal, encode_args};

// root_c
pub async fn root_create_canisters() -> Result<(), Error> {
    let root_pid = CanisterState::get_root_pid();

    if root_pid != canister_self() {
        return Err(InterfaceError::NotRoot)?;
    }

    // iterate canisters
    for (kind, data) in CanisterRegistry::export() {
        if data.attributes.auto_create && SubnetIndex::get(&kind).is_none() {
            root_canister_create(&kind, None).await.unwrap();
        }
    }

    subnet_index_cascade().await?;

    Ok(())
}

// root_canister_create
async fn root_canister_create(kind: &str, extra: Option<Vec<u8>>) -> Result<Principal, Error> {
    let canister = CanisterRegistry::try_get(kind)?;
    let root_pid = CanisterState::get_root_pid();

    // controllers are :
    // - the controllers that are specified in the config file
    let mut controllers = Config::get()?.controllers;
    controllers.push(root_pid);

    // encode the standard init args
    let this = CanisterParent::this()?;
    let parents = vec![this];
    let args = encode_args((parents, extra))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // create the canister
    let new_canister_id = ic_create_canister(kind, canister.wasm, controllers, args).await?;

    // always insert into the child index
    ChildIndex::insert(new_canister_id, kind);

    // optional - update subnet index
    if canister.attributes.indexable {
        SubnetIndex::insert(kind, new_canister_id);
    }

    Ok(new_canister_id)
}
