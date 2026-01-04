use crate::{Error, ops::ic::nns::registry::NnsRegistryOps, workflow::prelude::*};

/// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
pub async fn try_get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let subnet_id_opt = NnsRegistryOps::get_subnet_for_canister(canister_self()).await?;

    if let Some(subnet_id) = subnet_id_opt {
        log!(
            Topic::Topology,
            Info,
            "try_get_current_subnet_pid: {subnet_id}"
        );
    } else {
        log!(
            Topic::Topology,
            Warn,
            "try_get_current_subnet_pid: not found"
        );
    }

    Ok(subnet_id_opt)
}
