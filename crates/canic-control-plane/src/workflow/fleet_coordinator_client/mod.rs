//! Typed inter-Canister client for the Coordinator's role-owned command surface.
//!
//! This module carries only the exact Root-owned command fragments. It does not
//! own Coordinator state, operation advancement, fallback decoding, or protocol
//! negotiation.

use crate::dto::fleet_coordinator::CoordinatorRootRemovalOperationStatus;
use canic_contracts::{
    dto::{
        error::Error,
        fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
        fleet_registry::{
            FleetRegistry, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootSnapshotAcknowledgementRequest,
        },
    },
    protocol,
};
use canic_core::control_plane_support::{error::InternalError, ops::ic::call::CallOps};
use ic_platform::principal::ExternalPrincipal as Principal;

pub(super) async fn registry(coordinator: Principal) -> Result<FleetRegistry, InternalError> {
    let wire = protocol::registry_discovery::snapshot::METHOD_ID;
    let bytes =
        ic_wire::native::encode_request(wire, &(), protocol::registry_discovery::REQUEST_LIMITS)
            .map_err(|_| InternalError::invariant())?;
    let reply =
        CallOps::unbounded_wait(coordinator, protocol::registry_discovery::snapshot::METHOD)
            .with_raw_args(bytes)
            .execute()
            .await?
            .into_bytes();
    let response: Result<canic_contracts::dto::fleet_snapshot::FleetRegistry, Error> =
        ic_wire::native::decode_response(
            wire,
            &reply,
            protocol::registry_discovery::RESPONSE_LIMITS,
        )
        .map_err(|_| InternalError::invariant())?;
    response
        .map(Into::into)
        .map_err(InternalError::observed_public)
}

pub(super) async fn root_removal_status(
    coordinator: Principal,
    operation_id: [u8; 32],
) -> Result<CoordinatorRootRemovalOperationStatus, InternalError> {
    use canic_contracts::{dto::retirement_control, protocol::retirement_control as wire};
    let bytes = ic_wire::native::encode_request(
        wire::removal_status::METHOD_ID,
        &operation_id,
        wire::REQUEST_LIMITS,
    )
    .map_err(|_| InternalError::invariant())?;
    let reply = CallOps::unbounded_wait(coordinator, wire::removal_status::METHOD)
        .with_raw_args(bytes)
        .execute()
        .await?
        .into_bytes();
    let response: Result<retirement_control::RemovalStatus, Error> =
        ic_wire::native::decode_response(
            wire::removal_status::METHOD_ID,
            &reply,
            wire::RESPONSE_LIMITS,
        )
        .map_err(|_| InternalError::invariant())?;
    let status = response.map_err(InternalError::observed_public)?;
    if status.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    Ok(status.into())
}

pub(super) async fn acknowledge_root_snapshot(
    coordinator: Principal,
    request: FleetSubnetRootSnapshotAcknowledgementRequest,
) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
    use canic_contracts::{
        dto::coordinator_acknowledgements, protocol::coordinator_acknowledgements as wire,
    };
    let bytes = ic_wire::native::encode_request(
        wire::acknowledge::METHOD_ID,
        &coordinator_acknowledgements::AcknowledgeRequest::from(request),
        wire::acknowledge::REQUEST_LIMITS,
    )
    .map_err(|_| InternalError::invariant())?;
    let reply = CallOps::unbounded_wait(coordinator, wire::acknowledge::METHOD)
        .with_raw_args(bytes)
        .execute()
        .await?
        .into_bytes();
    let response: Result<coordinator_acknowledgements::RootAcknowledgement, Error> =
        ic_wire::native::decode_response(
            wire::acknowledge::METHOD_ID,
            &reply,
            wire::RESPONSE_LIMITS,
        )
        .map_err(|_| InternalError::invariant())?;
    response
        .map(Into::into)
        .map_err(InternalError::observed_public)
}

pub(super) async fn request_root_funding(
    coordinator: Principal,
    request: FleetRootFundingRequest,
) -> Result<FleetRootFundingResponse, InternalError> {
    use canic_contracts::{dto::funding_control, protocol::funding_control as wire};
    let bytes = ic_wire::native::encode_request(
        wire::request::METHOD_ID,
        &funding_control::Request::from(request),
        wire::REQUEST_LIMITS,
    )
    .map_err(|_| InternalError::invariant())?;
    let reply = CallOps::bounded_wait(coordinator, wire::request::METHOD)
        .with_raw_args(bytes)
        .execute()
        .await?
        .into_bytes();
    let response: Result<funding_control::Response, Error> =
        ic_wire::native::decode_response(wire::request::METHOD_ID, &reply, wire::RESPONSE_LIMITS)
            .map_err(|_| InternalError::invariant())?;
    response
        .map(Into::into)
        .map_err(InternalError::observed_public)
}
