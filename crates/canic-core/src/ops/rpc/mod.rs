pub mod methods;
pub mod request;

use crate::{
    Error, PublicError, ThisError,
    cdk::candid::CandidType,
    dto::rpc::{Request, Response},
    infra::InfraError,
    ops::{OpsError, ic::call::Call, rpc::request::RequestOpsError, runtime::env::EnvOps},
};
use serde::de::DeserializeOwned;

///
/// Rpc
/// Typed RPC command binding a request variant to its response payload.
///

pub trait Rpc {
    type Response: CandidType + DeserializeOwned;

    fn into_request(self) -> Request;
    fn try_from_response(resp: Response) -> Result<Self::Response, RequestOpsError>;
}

///
/// RpcOpsError
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] request::RequestOpsError),

    #[error("rpc rejected: {0:?}")]
    RemoteRejected(PublicError),
}

impl From<RpcOpsError> for Error {
    fn from(err: RpcOpsError) -> Self {
        OpsError::from(err).into()
    }
}

// execute_rpc
async fn execute_rpc<R: Rpc>(rpc: R) -> Result<R::Response, Error> {
    let root_pid = EnvOps::root_pid()?;

    let call_response = Call::unbounded_wait(root_pid, methods::CANIC_RESPONSE)
        .with_arg(rpc.into_request())
        .await
        .map_err(InfraError::from)?;

    // Boundary: convert RPC PublicError into internal Error.
    let response: Response = call_response
        .candid::<Result<Response, PublicError>>()
        .map_err(InfraError::from)?
        .map_err(RpcOpsError::RemoteRejected)?;

    let response = R::try_from_response(response).map_err(RpcOpsError::from)?;

    Ok(response)
}
