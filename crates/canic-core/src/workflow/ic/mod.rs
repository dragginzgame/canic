pub mod call;
pub mod ledger;
pub mod mgmt;
pub mod network;
pub mod provision;
pub mod signature;
pub mod xrc;

use crate::{
    InternalError,
    ops::ic::{IcOps, nns::registry::NnsRegistryOps},
    workflow::prelude::*,
};

///
/// IcWorkflow
///

pub struct IcWorkflow;

impl IcWorkflow {
    /// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
    pub async fn try_get_current_subnet_pid() -> Result<Option<Principal>, InternalError> {
        let subnet_id_opt = NnsRegistryOps::get_subnet_for_canister(IcOps::canister_self()).await?;

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
}
