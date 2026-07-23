//! Module: ops::ic::nns::registry
//!
//! Responsibility: query NNS registry topology information through approved ops APIs.
//! Does not own: subnet registry storage, topology workflow, or endpoint DTOs.
//! Boundary: delegates NNS registry query mechanics to infra.

use crate::{
    InternalError,
    infra::ic::nns::registry::NnsRegistryInfra,
    ops::{OpsError, prelude::*},
};

///
/// NnsRegistryOps
///
/// Operations-layer facade for NNS registry queries.
///

pub struct NnsRegistryOps;

impl NnsRegistryOps {
    pub async fn get_subnet_for_canister(
        pid: Principal,
    ) -> Result<Option<Principal>, InternalError> {
        let subnet = NnsRegistryInfra::get_subnet_for_canister(pid)
            .await
            .map_err(OpsError::from)?;

        Ok(subnet)
    }
}
