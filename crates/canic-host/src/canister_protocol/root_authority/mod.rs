//! Read the complete native Root authority without changing caller-owned artifact checks.
use super::CanisterProtocolError;
use crate::icp::IcpCli;
use canic_contracts::{
    dto::{fleet_subnet_root::FleetSubnetRootAuthority, root_authority::Authority},
    protocol::root_authority as wire,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Observe the selected Root through its exact native identity and bounded response.
pub fn query(
    icp: &IcpCli,
    root: Principal,
) -> Result<FleetSubnetRootAuthority, CanisterProtocolError> {
    let authority: Authority = super::query_wire_cli(
        icp,
        root,
        wire::METHOD,
        wire::METHOD_ID,
        &(),
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(authority.into())
}
