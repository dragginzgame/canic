//! Installed Coordinator discovery through the production native queries.
use canic::{
    Error,
    dto::{fleet_registry::FleetRegistryVersion, registry_discovery::RegistryVersion},
    protocol::registry_discovery,
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};
use ic_testkit::pic::{WireCallSpec, query_wire};

pub fn version(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
) -> Result<FleetRegistryVersion, Error> {
    version_as(pic, coordinator, ExternalPrincipal::anonymous())
}

pub fn version_as(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
) -> Result<FleetRegistryVersion, Error> {
    let reply: Result<RegistryVersion, Error> = query_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: registry_discovery::version::METHOD,
            method_id: registry_discovery::version::METHOD_ID,
            request_limits: registry_discovery::REQUEST_LIMITS,
            response_limits: registry_discovery::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native registry version transport");
    reply.map(Into::into)
}

#[cfg(test)]
pub fn manifest(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
) -> Result<canic::dto::fleet_registry::FleetRegistryManifest, Error> {
    let reply: Result<canic::dto::registry_discovery::RegistryManifest, Error> = query_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: registry_discovery::manifest::METHOD,
            method_id: registry_discovery::manifest::METHOD_ID,
            request_limits: registry_discovery::REQUEST_LIMITS,
            response_limits: registry_discovery::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native registry manifest transport");
    reply.map(Into::into)
}

/// Query as the actual observer; admission remains owned by the production endpoint.
pub fn snapshot(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
) -> Result<canic::dto::fleet_registry::FleetRegistry, Error> {
    let reply: Result<canic::dto::fleet_snapshot::FleetRegistry, Error> = query_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: registry_discovery::snapshot::METHOD,
            method_id: registry_discovery::snapshot::METHOD_ID,
            request_limits: registry_discovery::REQUEST_LIMITS,
            response_limits: registry_discovery::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native registry snapshot transport");
    reply.map(Into::into)
}

pub fn root_inventory(
    pic: &PocketIc,
    root: ExternalPrincipal,
) -> Result<canic::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary, Error> {
    let reply: Result<canic::dto::fleet_snapshot::RootInventory, Error> = query_wire(
        pic,
        root,
        ExternalPrincipal::anonymous(),
        WireCallSpec {
            method: registry_discovery::root_inventory::METHOD,
            method_id: registry_discovery::root_inventory::METHOD_ID,
            request_limits: registry_discovery::REQUEST_LIMITS,
            response_limits: registry_discovery::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native Root inventory transport");
    reply.map(Into::into)
}

/// Exercise the production native join update as its actual caller.
pub fn join(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::fleet_registry::FleetSubnetRootJoinRequest,
) -> Result<canic::dto::fleet_registry::FleetSubnetRootJoinResponse, Error> {
    use canic::{dto::registry_control, protocol::registry_control as wire};
    let response: Result<registry_control::JoinResponse, Error> = ic_testkit::pic::update_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: wire::join::METHOD,
            method_id: wire::join::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &registry_control::JoinRequest::from(request),
    )
    .expect("native registry mutation transport");
    response.map(Into::into)
}

/// Exercise the production native activate update as its actual caller.
pub fn activate(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::fleet_registry::FleetRegistryActivationRequest,
) -> Result<canic::dto::fleet_registry::FleetRegistryActivationResponse, Error> {
    use canic::{dto::registry_control, protocol::registry_control as wire};
    let response: Result<registry_control::ActivateResponse, Error> = ic_testkit::pic::update_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: wire::activate::METHOD,
            method_id: wire::activate::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &registry_control::ActivateRequest::from(request),
    )
    .expect("native registry mutation transport");
    response.map(Into::into)
}

/// Submit the original synchronization intent without manufacturing a new operation.
pub(super) fn synchronize(
    pic: &PocketIc,
    root: ExternalPrincipal,
    request: canic::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
) -> Result<canic::dto::role::OperationReceipt, canic::Error> {
    use canic::protocol::registry_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        ExternalPrincipal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::synchronize::METHOD,
            method_id: wire::synchronize::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &canic::dto::registry_control::SynchronizeRequest::from(request),
    )
    .expect("native Root registry synchronization transport")
}

/// Observe both synchronization and mirror activation without advancing either.
pub fn synchronization_status(
    pic: &PocketIc,
    root: ExternalPrincipal,
    operation_id: [u8; 32],
) -> Result<canic::dto::root::RootRegistrySynchronizationOperationStatus, Error> {
    use canic::{dto::registry_control, protocol::registry_control as wire};
    let response: Result<registry_control::SynchronizationStatus, Error> = query_wire(
        pic,
        root,
        ExternalPrincipal::anonymous(),
        WireCallSpec {
            method: wire::synchronization_status::METHOD,
            method_id: wire::synchronization_status::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &operation_id,
    )
    .expect("native Root synchronization observation transport");
    response.map(|native| {
        let canonical =
            canic::dto::root::RootRegistrySynchronizationOperationStatus::from(native.clone());
        assert_eq!(
            registry_control::SynchronizationStatus::from(canonical.clone()),
            native
        );
        canonical
    })
}
