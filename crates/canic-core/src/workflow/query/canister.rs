use crate::{
    Error,
    cdk::{mgmt::CanisterStatusResult, types::Principal},
    ops::ic::mgmt,
};

pub(crate) async fn canister_status(pid: Principal) -> Result<CanisterStatusResult, Error> {
    mgmt::canister_status(pid).await
}
