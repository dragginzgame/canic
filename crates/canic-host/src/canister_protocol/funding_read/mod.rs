//! Native funding observation; policy decisions and mutation remain caller-owned.
use super::{CanisterProtocolError, query_wire_cli};
use crate::icp::{IcpCli, IcpWireQueryError};
use canic_contracts::{
    dto::{funding_read::RootStatus, root::RootFundingStatusResponse},
    protocol::funding_read as wire,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Read the complete Root funding evidence through the selected operator identity.
pub fn root_status(
    icp: &IcpCli,
    root: Principal,
) -> Result<RootFundingStatusResponse, CanisterProtocolError> {
    let status: RootStatus = query_wire_cli(
        icp,
        root,
        wire::root_status::METHOD,
        wire::root_status::METHOD_ID,
        &(),
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    status
        .try_into()
        .map_err(|source| CanisterProtocolError::WireQuery {
            canister: root,
            method: wire::root_status::METHOD,
            source: IcpWireQueryError::Wire(source),
        })
}

/// Observe treasury and Root accounting through the selected operator identity.
pub fn coordinator_status(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<
    canic_contracts::dto::fleet_coordinator::CoordinatorFundingStatusResponse,
    CanisterProtocolError,
> {
    let status: canic_contracts::dto::funding_read::CoordinatorStatus = query_wire_cli(
        icp,
        coordinator,
        wire::coordinator_status::METHOD,
        wire::coordinator_status::METHOD_ID,
        &(),
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(status.into())
}
