pub mod ledger;
pub mod mgmt;
pub mod provision;
pub mod xrc;

use crate::{
    Error, ThisError,
    ops::ic::nns::registry::NnsRegistryOps,
    workflow::{WorkflowError, prelude::*},
};

///
/// IcWorkflowError
///

#[derive(Debug, ThisError)]
pub enum IcWorkflowError {
    #[error(transparent)]
    LedgerWorkflow(#[from] ledger::LedgerWorkflowError),

    #[error(transparent)]
    ProvisionWorkflow(#[from] provision::ProvisionWorkflowError),
}

impl From<IcWorkflowError> for Error {
    fn from(err: IcWorkflowError) -> Self {
        WorkflowError::from(err).into()
    }
}

///
/// IcWorkflow
///

pub struct IcWorkflow;

impl IcWorkflow {
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
}
