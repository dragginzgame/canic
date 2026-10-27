//! Single bounded Coordinator-to-Root admission calls; workflow owns durable progress and retries.
use canic_contracts::{
    dto::{
        admission_control as native,
        error::Error,
        fleet_admission::{
            FleetAdmissionActivateRootRequest, FleetAdmissionOpenRootRequest,
            FleetAdmissionPrepareRootRequest, FleetAdmissionRootReceipt,
        },
    },
    protocol::admission_control as protocol,
};
use canic_core::control_plane_support::{error::InternalError, ops::ic::call::CallOps};
use ic_platform::principal::ExternalPrincipal;

pub async fn prepare(
    root: ExternalPrincipal,
    request: FleetAdmissionPrepareRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    call(
        root,
        protocol::root_prepare::METHOD,
        protocol::root_prepare::METHOD_ID,
        &native::PrepareRootRequest::from(request),
    )
    .await
}

pub async fn activate(
    root: ExternalPrincipal,
    request: FleetAdmissionActivateRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    call(
        root,
        protocol::root_activate::METHOD,
        protocol::root_activate::METHOD_ID,
        &native::ActivateRootRequest::from(request),
    )
    .await
}

pub async fn open(
    root: ExternalPrincipal,
    request: FleetAdmissionOpenRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    call(
        root,
        protocol::root_open::METHOD,
        protocol::root_open::METHOD_ID,
        &native::OpenRootRequest::from(request),
    )
    .await
}

async fn call<T: serde::Serialize + Sync>(
    root: ExternalPrincipal,
    method: &str,
    id: u64,
    request: &T,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    let bytes = ic_wire::native::encode_request(id, request, protocol::REQUEST_LIMITS)
        .map_err(|_| InternalError::invariant())?;
    let reply = CallOps::bounded_wait(root, method)
        .with_raw_args(bytes)
        .execute()
        .await?
        .into_bytes();
    let result: Result<native::RootReceipt, Error> =
        ic_wire::native::decode_response(id, &reply, protocol::RESPONSE_LIMITS)
            .map_err(|_| InternalError::invariant())?;
    result
        .map(Into::into)
        .map_err(InternalError::observed_public)
}
