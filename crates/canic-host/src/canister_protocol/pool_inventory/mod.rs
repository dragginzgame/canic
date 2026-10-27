//! Native pool observation; inventory validation and cache lifetime remain caller-owned.
use super::{CanisterProtocolError, query_wire_cli};
use crate::icp::IcpCli;
use canic_contracts::{
    dto::{
        pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
        pool_inventory::{Page, Request},
    },
    protocol::pool_inventory as wire,
};
use ic_platform::principal::ExternalPrincipal as Principal;
/// Observe one exact page without changing cursor, admission or retained authority.
pub fn query(
    icp: &IcpCli,
    root: Principal,
    request: &CanisterPoolStatusRequest,
) -> Result<CanisterPoolResponse, CanisterProtocolError> {
    let request = Request::from(*request);
    let page: Page = query_wire_cli(
        icp,
        root,
        wire::METHOD,
        wire::METHOD_ID,
        &request,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(page.into())
}
