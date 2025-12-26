pub(crate) mod methods;
mod request;
mod types;

pub use request::*;
pub use types::*;

use crate::{
    Error, ThisError,
    cdk::candid::CandidType,
    ops::{OpsError, env::EnvOps, ic::call::Call},
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
    RequestOpsError(#[from] request::RequestOpsError),
}

impl From<RpcOpsError> for Error {
    fn from(err: RpcOpsError) -> Self {
        OpsError::from(err).into()
    }
}

// execute_rpc
async fn execute_rpc<R: Rpc>(rpc: R) -> Result<R::Response, Error> {
    let root_pid = EnvOps::root_pid();

    let call_response = Call::unbounded_wait(root_pid, methods::CANIC_RESPONSE)
        .with_arg(rpc.into_request())
        .await?;

    let response = call_response.candid::<Result<Response, Error>>()??;

    R::try_from_response(response)
        .map_err(RpcOpsError::from)
        .map_err(Error::from)
}
