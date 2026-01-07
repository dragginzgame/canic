use crate::{
    PublicError, cdk::types::Principal, dto::canister::CanisterStatusView,
    workflow::ic::mgmt::MgmtWorkflow,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusView, PublicError> {
        MgmtWorkflow::canister_status_view(pid)
            .await
            .map_err(PublicError::from)
    }
}
