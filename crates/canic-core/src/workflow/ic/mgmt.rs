use crate::{
    InternalError, dto::canister::CanisterStatusResponse, ops::ic::mgmt::MgmtOps,
    workflow::prelude::*,
};

///
/// MgmtWorkflow
///

pub struct MgmtWorkflow;

impl MgmtWorkflow {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, InternalError> {
        let status = MgmtOps::canister_status(pid).await?;

        Ok(MgmtOps::canister_status_to_dto(status))
    }
}
