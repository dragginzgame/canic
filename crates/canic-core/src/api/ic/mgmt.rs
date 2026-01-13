use crate::{
    cdk::types::Principal,
    dto::{canister::CanisterStatusView, error::Error},
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusView, Error> {
        MgmtWorkflow::canister_status_view(pid)
            .await
            .map_err(Error::from)
    }
}
