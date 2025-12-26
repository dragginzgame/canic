use crate::{
    Error,
    env::nns::NNS_REGISTRY_CANISTER,
    log,
    log::Topic,
    ops::ic::call::Call,
    spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
};
use candid::{Principal, decode_one, encode_args, utils::ArgumentEncoder};

//
// ────────────────────────────── TOPOLOGY LOOKUPS ─────────────────────────────
//

/// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
pub async fn try_get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let request = GetSubnetForCanisterRequest::new(crate::cdk::api::canister_self());

    let subnet_id_opt = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
        .with_arg(request)
        .await?
        .candid::<GetSubnetForCanisterResponse>()?
        .map_err(Error::CallFailed)?
        .subnet_id;

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
