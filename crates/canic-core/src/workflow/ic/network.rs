use crate::{
    Error, cdk::api::canister_self, infra::ic::nns::registry::get_subnet_for_canister, log,
    log::Topic,
};
use candid::Principal;

//
// ────────────────────────────── TOPOLOGY LOOKUPS ─────────────────────────────
//

/// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
pub(crate) async fn try_get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let subnet_id_opt = get_subnet_for_canister(canister_self()).await?;

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
