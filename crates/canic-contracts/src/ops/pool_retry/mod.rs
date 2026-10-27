//! Convert the native retry selection into the existing pool command without changing identity.
use crate::dto::{pool::PoolAdminCommand, pool_retry::ResetRequest};

impl From<ResetRequest> for PoolAdminCommand {
    fn from(value: ResetRequest) -> Self {
        Self::RetryReset {
            canister_id: value.canister_id.into(),
        }
    }
}

impl From<ic_platform::principal::ExternalPrincipal> for crate::dto::pool_retry::ResetResponse {
    fn from(value: ic_platform::principal::ExternalPrincipal) -> Self {
        Self {
            canister_id: value.into(),
        }
    }
}
