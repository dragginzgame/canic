//! Native registry reads; callers retain artifact, inventory and authority validation.
use super::{CanisterProtocolError, query_wire_cli};
use crate::icp::IcpCli;
use canic_contracts::{
    dto::{component_control, component_registry as stored, registry_read},
    protocol::registry_read as wire,
};
use ic_platform::principal::ExternalPrincipal as Principal;
/// Observe the exact status through the selected operator identity.
pub fn status(
    icp: &IcpCli,
    root: Principal,
    request: stored::RootComponentRegistryPreparationRequest,
) -> Result<stored::RootComponentRegistryStatusResponse, CanisterProtocolError> {
    let request: component_control::PreparationRequest = request.into();
    let response: component_control::RegistryStatus = query_wire_cli(
        icp,
        root,
        wire::status::METHOD,
        wire::status::METHOD_ID,
        &request,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(response.into())
}
/// Observe the exact partition through the selected operator identity.
pub fn partition(
    icp: &IcpCli,
    root: Principal,
    request: stored::ComponentRegistryPartitionRequest,
) -> Result<stored::ComponentRegistryPartitionResponse, CanisterProtocolError> {
    let response: registry_read::Partition = query_wire_cli(
        icp,
        root,
        wire::partition::METHOD,
        wire::partition::METHOD_ID,
        &request,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(response.into())
}
/// Observe the exact active partition through the selected operator identity.
pub fn active_partition(
    icp: &IcpCli,
    root: Principal,
    request: stored::ComponentRegistryActivePartitionRequest,
) -> Result<stored::ComponentRegistryActivePartitionResponse, CanisterProtocolError> {
    let response: registry_read::ActivePartition = query_wire_cli(
        icp,
        root,
        wire::active_partition::METHOD,
        wire::active_partition::METHOD_ID,
        &request,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    Ok(response.into())
}
