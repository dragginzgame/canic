use crate::{
    Error,
    cdk::{mgmt::CanisterStatusResult, types::Principal},
    ops::ic::mgmt,
};

/// Read-only IC management query.
/// Delegates to ops::ic::mgmt and performs no lifecycle or policy logic.
pub(crate) async fn canister_status(pid: Principal) -> Result<CanisterStatusResult, Error> {
    mgmt::canister_status(pid).await
}
