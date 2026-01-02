use crate::{Error, cdk::types::Principal, dto::canister::CanisterStatusView, ops::ic::mgmt};

/// Read-only IC management query.
/// Delegates to ops::ic::mgmt and performs no lifecycle or policy logic.
pub(crate) async fn canister_status(pid: Principal) -> Result<CanisterStatusView, Error> {
    mgmt::canister_status_view(pid).await
}
