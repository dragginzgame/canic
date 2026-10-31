//! Single bounded target admission calls; workflow owns retries and durable receipt recording.
use canic_contracts::{
    dto::{
        admission_control as native,
        error::Error,
        fleet_admission::{
            FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
            FleetAdmissionPrepareTargetRequest, FleetAdmissionTargetReceipt,
            FleetAdmissionTargetTransitionPhase,
        },
    },
    protocol::admission_control as protocol,
};
use canic_core::control_plane_support::{
    error::InternalError, ops::ic::call::execute as execute_call,
};
use ic_call::{Call, WaitPolicy};
use ic_platform::principal::ExternalPrincipal;

pub async fn prepare(
    target: ExternalPrincipal,
    request: FleetAdmissionPrepareTargetRequest,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    call(
        target,
        protocol::target_prepare::METHOD,
        protocol::target_prepare::METHOD_ID,
        &native::PrepareTargetRequest::from(request),
        FleetAdmissionTargetTransitionPhase::Prepare,
    )
    .await
}

pub async fn activate(
    target: ExternalPrincipal,
    request: FleetAdmissionActivateTargetRequest,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    call(
        target,
        protocol::target_activate::METHOD,
        protocol::target_activate::METHOD_ID,
        &request,
        FleetAdmissionTargetTransitionPhase::Activate,
    )
    .await
}

pub async fn open(
    target: ExternalPrincipal,
    request: FleetAdmissionOpenTargetRequest,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    call(
        target,
        protocol::target_open::METHOD,
        protocol::target_open::METHOD_ID,
        &request,
        FleetAdmissionTargetTransitionPhase::Open,
    )
    .await
}

async fn call<T: serde::Serialize + Sync>(
    target: ExternalPrincipal,
    method: &str,
    id: u64,
    request: &T,
    phase: FleetAdmissionTargetTransitionPhase,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    let bytes = ic_wire::native::encode_request(id, request, protocol::REQUEST_LIMITS)
        .map_err(|_| InternalError::invariant())?;
    let reply = execute_call(
        Call::new(
            target.into(),
            method,
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(bytes),
    )
    .await?
    .into_bytes();
    let result: Result<native::TargetReceipt, Error> =
        ic_wire::native::decode_response(id, &reply, protocol::RESPONSE_LIMITS)
            .map_err(|_| InternalError::invariant())?;
    let receipt = result.map_err(InternalError::observed_public)?;
    if receipt.phase != phase {
        return Err(InternalError::conflict());
    }
    Ok(receipt.into())
}
