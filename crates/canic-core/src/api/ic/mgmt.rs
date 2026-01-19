use crate::{
    cdk::types::Principal,
    dto::{canister::CanisterStatusResponse, error::Error},
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, Error> {
        MgmtWorkflow::canister_status(pid)
            .await
            .map_err(Error::from)
    }
}
