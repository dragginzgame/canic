use crate::{
    Error, infra,
    ops::{ic::IcOpsError, prelude::*},
};

///
/// NnsRegistryOps
///

pub struct NnsRegistryOps;

impl NnsRegistryOps {
    pub async fn get_subnet_for_canister(pid: Principal) -> Result<Option<Principal>, Error> {
        let subnet = infra::ic::nns::registry::get_subnet_for_canister(pid)
            .await
            .map_err(IcOpsError::from)?;

        Ok(subnet)
    }
}
