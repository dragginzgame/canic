use crate::{Error, infra::ic::nns::registry as infra_registry};
use candid::Principal;

pub async fn get_subnet_for_canister(pid: Principal) -> Result<Option<Principal>, Error> {
    infra_registry::get_subnet_for_canister(pid)
        .await
        .map_err(Error::from)
}
