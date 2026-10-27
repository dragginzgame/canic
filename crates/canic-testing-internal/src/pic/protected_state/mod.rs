//! Shared installed calls for typed, controller-protected authority state.
use canic::Error;
#[cfg(test)]
use canic::{
    dto::{authority_restore_status::Status, state::FleetStateResponse},
    protocol::protected_state,
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};
use ic_testkit::pic::{WireCallSpec, query_wire};

#[cfg(test)]
pub fn authority_restore(
    pic: &PocketIc,
    canister: ExternalPrincipal,
    caller: ExternalPrincipal,
) -> Result<Status, Error> {
    query_wire(
        pic,
        canister,
        caller,
        WireCallSpec {
            method: protected_state::authority_restore::METHOD,
            method_id: protected_state::authority_restore::METHOD_ID,
            request_limits: protected_state::REQUEST_LIMITS,
            response_limits: protected_state::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native authority restore status")
}

#[cfg(test)]
pub fn fleet_state(
    pic: &PocketIc,
    root: ExternalPrincipal,
    caller: ExternalPrincipal,
) -> Result<FleetStateResponse, Error> {
    query_wire(
        pic,
        root,
        caller,
        WireCallSpec {
            method: protected_state::fleet_state::METHOD,
            method_id: protected_state::fleet_state::METHOD_ID,
            request_limits: protected_state::REQUEST_LIMITS,
            response_limits: protected_state::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native Root Fleet state")
}

/// Invoke the production snapshot prepare endpoint as the selected controller.
#[cfg(test)]
pub fn prepare_snapshot(
    pic: &PocketIc,
    authority: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::authority_restore::AuthoritySnapshotRequest,
) -> Result<Status, Error> {
    ic_testkit::pic::update_wire(
        pic,
        authority,
        caller,
        WireCallSpec {
            method: protected_state::snapshot_prepare::METHOD,
            method_id: protected_state::snapshot_prepare::METHOD_ID,
            request_limits: protected_state::snapshot_prepare::REQUEST_LIMITS,
            response_limits: protected_state::snapshot_prepare::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native authority snapshot prepare")
}

/// Invoke the production snapshot resume endpoint as the selected controller.
#[cfg(test)]
pub fn resume_snapshot(
    pic: &PocketIc,
    authority: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::authority_restore::AuthoritySnapshotRequest,
) -> Result<Status, Error> {
    ic_testkit::pic::update_wire(
        pic,
        authority,
        caller,
        WireCallSpec {
            method: protected_state::snapshot_resume::METHOD,
            method_id: protected_state::snapshot_resume::METHOD_ID,
            request_limits: protected_state::snapshot_resume::REQUEST_LIMITS,
            response_limits: protected_state::snapshot_resume::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native authority snapshot resume")
}

/// Invoke the production controller-owned Fleet funding mutation.
#[cfg(test)]
pub fn set_funding(
    pic: &PocketIc,
    root: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::state::SetCyclesFundingRequest,
) -> Result<canic::dto::state::FleetStateCommandResult<bool>, Error> {
    use canic::protocol::fleet_state_control as protocol;
    let response: Result<canic::dto::fleet_state_control::CommandResult<bool>, Error> =
        ic_testkit::pic::update_wire(
            pic,
            root,
            caller,
            WireCallSpec {
                method: protocol::funding::METHOD,
                method_id: protocol::funding::METHOD_ID,
                request_limits: protocol::REQUEST_LIMITS,
                response_limits: protocol::RESPONSE_LIMITS,
            },
            &request,
        )
        .expect("native Fleet funding mutation");
    response.map(Into::into)
}

/// Invoke the production controller-owned Fleet status mutation.
#[cfg(test)]
pub fn set_status(
    pic: &PocketIc,
    root: ExternalPrincipal,
    caller: ExternalPrincipal,
    request: canic::dto::state::SetFleetStatusRequest,
) -> Result<canic::dto::state::FleetStateCommandResult<canic::dto::state::FleetStatus>, Error> {
    use canic::protocol::fleet_state_control as protocol;
    let response: Result<
        canic::dto::fleet_state_control::CommandResult<canic::dto::state::FleetStatus>,
        Error,
    > = ic_testkit::pic::update_wire(
        pic,
        root,
        caller,
        WireCallSpec {
            method: protocol::status::METHOD,
            method_id: protocol::status::METHOD_ID,
            request_limits: protocol::REQUEST_LIMITS,
            response_limits: protocol::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native Fleet status mutation");
    response.map(Into::into)
}

/// Read one exact managed runtime operation through its production native endpoint.
pub fn runtime_operation(
    pic: &PocketIc,
    target: ExternalPrincipal,
    root: ExternalPrincipal,
    operation_id: [u8; 32],
) -> Result<
    Result<canic::dto::role::ComponentRuntimeOperationStatus, Error>,
    ic_testkit::pic::WireCallError,
> {
    use canic::protocol::runtime_reporting as wire;
    let response: Result<canic::dto::runtime_reporting::ComponentRuntimeOperationStatus, Error> =
        query_wire(
            pic,
            target,
            root,
            WireCallSpec {
                method: wire::METHOD,
                method_id: wire::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &canic::dto::role::OperationStatusRequest { operation_id },
        )?;
    Ok(response.map(Into::into))
}
