//! Typed registry reads after exact installed-release schema admission.
use crate::{
    canister_protocol::{CanisterProtocolError, query_wire},
    icp::IcpCli,
    release_set::BuiltWireSchema,
};
use canic_contracts::{
    dto::{fleet_registry, registry_discovery as native},
    protocol::registry_discovery as protocol,
};
use ic_platform::principal::ExternalPrincipal;

/// Read the current registry manifest using the admitted release schema.
pub fn manifest(
    icp: &IcpCli,
    _schema: &BuiltWireSchema,
    coordinator: ExternalPrincipal,
) -> Result<fleet_registry::FleetRegistryManifest, CanisterProtocolError> {
    let value: native::RegistryManifest = query_wire(
        icp,
        coordinator,
        protocol::manifest::METHOD,
        protocol::manifest::METHOD_ID,
        &(),
        protocol::REQUEST_LIMITS,
        protocol::RESPONSE_LIMITS,
    )?;
    Ok(value.into())
}

/// Read the current registry version using the admitted release schema.
pub fn version(
    icp: &IcpCli,
    _schema: &BuiltWireSchema,
    coordinator: ExternalPrincipal,
) -> Result<fleet_registry::FleetRegistryVersion, CanisterProtocolError> {
    let value: native::RegistryVersion = query_wire(
        icp,
        coordinator,
        protocol::version::METHOD,
        protocol::version::METHOD_ID,
        &(),
        protocol::REQUEST_LIMITS,
        protocol::RESPONSE_LIMITS,
    )?;
    Ok(value.into())
}

/// Read the canonical snapshot. Callers own custody, release selection and hash validation;
/// this transport observation never authorizes a mutation or a paid effect.
pub fn snapshot(
    icp: &IcpCli,
    coordinator: ExternalPrincipal,
) -> Result<fleet_registry::FleetRegistry, CanisterProtocolError> {
    let value: canic_contracts::dto::fleet_snapshot::FleetRegistry = query_wire(
        icp,
        coordinator,
        protocol::snapshot::METHOD,
        protocol::snapshot::METHOD_ID,
        &(),
        protocol::REQUEST_LIMITS,
        protocol::RESPONSE_LIMITS,
    )?;
    Ok(value.into())
}

/// Read one controller-protected Root summary; callers validate its exact active head.
pub fn root_inventory(
    icp: &IcpCli,
    root: ExternalPrincipal,
) -> Result<
    canic_contracts::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary,
    CanisterProtocolError,
> {
    let value: canic_contracts::dto::fleet_snapshot::RootInventory = query_wire(
        icp,
        root,
        protocol::root_inventory::METHOD,
        protocol::root_inventory::METHOD_ID,
        &(),
        protocol::REQUEST_LIMITS,
        protocol::RESPONSE_LIMITS,
    )?;
    Ok(value.into())
}
