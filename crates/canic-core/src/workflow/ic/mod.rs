//! Module: workflow::ic
//!
//! Responsibility: coordinate IC-facing workflow helpers and registry lookups.
//! Does not own: IC call execution, endpoint authorization, or stable storage.
//! Boundary: exposes workflow facades over IC ops and build-network metadata.

pub mod build_network;
pub mod call;
pub mod icp_refill;
pub mod mgmt;
pub mod provision;

use crate::{
    InternalError,
    cdk::types::Principal,
    log,
    log::Topic,
    ops::ic::{IcOps, nns::registry::NnsRegistryOps},
};

///
/// IcWorkflow
///
/// Workflow facade for IC registry and build-network metadata operations.
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
