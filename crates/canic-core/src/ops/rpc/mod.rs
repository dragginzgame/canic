pub mod cascade;
pub mod methods;
pub mod request;

use crate::{
    Error, PublicError, ThisError,
    cdk::candid::{CandidType, Principal},
    dto::rpc::{Request, Response},
    infra::InfraError,
    ops::{OpsError, ic::call::Call, rpc::request::RequestOpsError, runtime::env::EnvOps},
};
use serde::de::DeserializeOwned;

///
/// RpcOpsError
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] request::RequestOpsError),

    // NOTE: PublicError is a wire-level contract only.
    // It is fully consumed and erased at the ops boundary.
    #[error("rpc rejected: {0:?}")]
    RemoteRejected(String),
}

impl From<RpcOpsError> for Error {
    fn from(err: RpcOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// Rpc
/// Typed RPC command binding a request variant to its response payload.
///

pub trait Rpc {
    type Response: CandidType + DeserializeOwned;

    fn into_request(self) -> Request;
    fn try_from_response(resp: Response) -> Result<Self::Response, RequestOpsError>;
}

// call_rpc_result
pub async fn call_rpc_result<T>(
    pid: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, Error>
where
    T: CandidType + for<'de> candid::Deserialize<'de>,
{
    let call = Call::unbounded_wait(pid, method)
        .with_arg(arg)
        .execute()
        .await
        .map_err(InfraError::from)?;

    // Explicit decode target is required
    let res: Result<T, PublicError> = call
        .candid::<Result<T, PublicError>>()
        .map_err(InfraError::from)?;

    // PublicError is consumed HERE, in ops
    res.map_err(|err| RpcOpsError::RemoteRejected(err.to_string()).into())
}

// execute_root_response_rpc
async fn execute_root_response_rpc<R: Rpc>(rpc: R) -> Result<R::Response, Error> {
    let root_pid = EnvOps::root_pid()?;

    let call_response = Call::unbounded_wait(root_pid, methods::CANIC_RESPONSE)
        .with_arg(rpc.into_request())
        .execute()
        .await
        .map_err(InfraError::from)?;

    // Boundary: convert RPC PublicError into internal Error.
    let response: Response = call_response
        .candid::<Result<Response, PublicError>>()
        .map_err(InfraError::from)?
        .map_err(|err| RpcOpsError::RemoteRejected(err.to_string()))?;

    let response = R::try_from_response(response).map_err(RpcOpsError::from)?;

    Ok(response)
}
