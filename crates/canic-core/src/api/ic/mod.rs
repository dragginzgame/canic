pub mod http;
pub mod signature;

use crate::{PublicError, cdk::types::Principal, dto::canister::CanisterStatusView, ops};

pub async fn canister_status(pid: Principal) -> Result<CanisterStatusView, PublicError> {
    ops::ic::mgmt::canister_status(pid)
        .await
        .map_err(PublicError::from)
}
