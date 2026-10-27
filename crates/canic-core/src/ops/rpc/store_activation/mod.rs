//! Transport Store activation controls without changing their retained authority or wait mode.
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        error::Error,
        fleet_activation::{FleetActivationRequest, FleetCredentialGenerationRequest},
        role::OperationReceipt,
    },
    ops::ic::call::CallOps,
};
use canic_contracts::protocol::store_activation as wire;

/// Prepare the existing credential generation through an unbounded IC call.
pub async fn prepare(
    pid: Principal,
    request: FleetCredentialGenerationRequest,
) -> Result<OperationReceipt, InternalError> {
    send(
        pid,
        wire::prepare::METHOD,
        wire::prepare::METHOD_ID,
        request,
    )
    .await
}
/// Activate the existing operation through an unbounded IC call.
pub async fn activate(
    pid: Principal,
    request: FleetActivationRequest,
) -> Result<OperationReceipt, InternalError> {
    send(
        pid,
        wire::activate::METHOD,
        wire::activate::METHOD_ID,
        request,
    )
    .await
}
async fn send(
    pid: Principal,
    method: &str,
    id: u64,
    request: impl serde::Serialize,
) -> Result<OperationReceipt, InternalError> {
    let payload = ic_wire::native::encode_request(id, &request, wire::REQUEST_LIMITS)
        .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_FAILED))?;
    let bytes = CallOps::unbounded_wait(pid, method)
        .with_raw_args(payload)
        .execute()
        .await?
        .into_bytes();
    ic_wire::native::decode_response::<Result<OperationReceipt, Error>>(
        id,
        &bytes,
        wire::RESPONSE_LIMITS,
    )
    .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_INVALID))?
    .map_err(InternalError::observed_public)
}

/// Read the exact activation operation with the original unbounded inter-canister wait.
pub async fn status(
    pid: Principal,
    operation_id: [u8; 32],
) -> Result<crate::dto::fleet_activation::FleetActivationStatusResponse, InternalError> {
    use canic_contracts::protocol::store_status as status;
    let bytes = ic_wire::native::encode_request(
        status::activation::METHOD_ID,
        &crate::dto::role::OperationStatusRequest { operation_id },
        status::REQUEST_LIMITS,
    )
    .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_FAILED))?;
    let bytes = CallOps::unbounded_wait(pid, status::activation::METHOD)
        .with_raw_args(bytes)
        .execute()
        .await?
        .into_bytes();
    let reply: Result<
        canic_contracts::dto::runtime_reporting::FleetActivationStatusResponse,
        Error,
    > = ic_wire::native::decode_response(
        status::activation::METHOD_ID,
        &bytes,
        status::RESPONSE_LIMITS,
    )
    .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_INVALID))?;
    reply
        .map(Into::into)
        .map_err(InternalError::observed_public)
}
