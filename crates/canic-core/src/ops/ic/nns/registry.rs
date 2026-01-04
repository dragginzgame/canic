use crate::{Error, infra, ops::prelude::*};

pub async fn get_subnet_for_canister(pid: Principal) -> Result<Option<Principal>, Error> {
    infra::ic::nns::registry::get_subnet_for_canister(pid)
        .await
        .map_err(Error::from)
}
