use crate::{
    PublicError, cdk::types::Principal, dto::canister::CanisterStatusView, ops::ic::mgmt::MgmtOps,
};

///
/// MgmtApi
///

pub struct MgmtApi;

impl MgmtApi {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusView, PublicError> {
        MgmtOps::canister_status(pid)
            .await
            .map_err(PublicError::from)
    }
}
