use crate::{
    InternalError,
    infra::ic::nns::registry::NnsRegistryInfra,
    ops::{ic::IcOpsError, prelude::*},
};

///
/// NnsRegistryOps
///

pub struct NnsRegistryOps;

impl NnsRegistryOps {
    pub async fn get_subnet_for_canister(
        pid: Principal,
    ) -> Result<Option<Principal>, InternalError> {
        let subnet = NnsRegistryInfra::get_subnet_for_canister(pid)
            .await
            .map_err(IcOpsError::from)?;

        Ok(subnet)
    }
}
