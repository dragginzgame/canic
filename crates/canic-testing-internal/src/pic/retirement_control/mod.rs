//! Installed lifecycle callers preserve the original canonical retirement evidence.
use canic::{
    Error,
    dto::{fleet_registry, retirement_control},
    protocol::retirement_control as wire,
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};
use ic_testkit::pic::{WireCallSpec, update_wire};

/// Submit the exact removal reservation using the selected controller identity.
pub(super) fn remove(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: fleet_registry::FleetSubnetRootDrainingReservationRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    update_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: wire::remove::METHOD,
            method_id: wire::remove::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &retirement_control::RemovalRequest::from(request),
    )
    .expect("native Root removal transport")
}

/// Reconcile the original Ledger transfer without replacement issuance.
pub(super) fn retire(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: fleet_registry::FleetRetirementRequest,
) -> Result<fleet_registry::FleetRetirementStatus, Error> {
    let response: Result<retirement_control::RetirementStatus, Error> = update_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: wire::retire::METHOD,
            method_id: wire::retire::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &retirement_control::RetirementRequest::from(request),
    )
    .expect("native Fleet retirement transport");
    response.map(Into::into)
}

/// Read retained retirement evidence without retrying the Ledger transfer.
pub(super) fn status(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    operation_id: [u8; 32],
) -> Result<fleet_registry::FleetRetirementStatus, Error> {
    let response: Result<retirement_control::RetirementStatus, Error> =
        ic_testkit::pic::query_wire(
            pic,
            coordinator,
            caller,
            WireCallSpec {
                method: wire::status::METHOD,
                method_id: wire::status::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native retirement observation transport");
    response.map(Into::into)
}

/// Read the original removal receipts without advancing lifecycle effects.
pub(super) fn removal_status(
    pic: &PocketIc,
    coordinator: ExternalPrincipal,
    caller: ExternalPrincipal,
    operation_id: [u8; 32],
) -> Result<canic::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus, Error> {
    let response: Result<retirement_control::RemovalStatus, Error> = ic_testkit::pic::query_wire(
        pic,
        coordinator,
        caller,
        WireCallSpec {
            method: wire::removal_status::METHOD,
            method_id: wire::removal_status::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &operation_id,
    )
    .expect("native Root removal observation transport");
    response.map(Into::into)
}
