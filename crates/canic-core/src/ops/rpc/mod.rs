pub mod request;

use crate::{
    Error, PublicError, ThisError,
    dto::rpc::{Request, Response},
    ops::{
        OpsError,
        ic::call::{CallOps, CallResult},
        prelude::*,
        rpc::request::RequestOpsError,
        runtime::env::EnvOps,
    },
    protocol,
};
use serde::de::DeserializeOwned;

///
/// RpcOpsError
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] RequestOpsError),

    // PublicError is a wire-level contract only.
    // It is erased at the ops boundary.
    #[error("rpc rejected: {0}")]
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

///
/// RpcOps
///

pub struct RpcOps;

impl RpcOps {
    ///
    /// call_rpc_result
    ///
    /// Calls a method that returns `Result<T, PublicError>` and
    /// erases `PublicError` at the ops boundary.
    ///
    pub async fn call_rpc_result<T>(
        pid: Principal,
        method: &str,
        arg: impl CandidType,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
    {
        let call: CallResult = CallOps::unbounded_wait(pid, method)
            .try_with_arg(arg)?
            .execute()
            .await?;

        let res: Result<T, PublicError> = call.candid::<Result<T, PublicError>>()?;

        res.map_err(|err| RpcOpsError::RemoteRejected(err.to_string()).into())
    }

    ///
    /// execute_root_response_rpc
    ///
    /// Executes a protocol-level RPC via Request/Response.
    ///
    async fn execute_root_response_rpc<R: Rpc>(rpc: R) -> Result<R::Response, Error> {
        let root_pid = EnvOps::root_pid()?;

        let call: CallResult = CallOps::unbounded_wait(root_pid, protocol::CANIC_RESPONSE)
            .try_with_arg(rpc.into_request())?
            .execute()
            .await?;

        let response: Response = call
            .candid::<Result<Response, PublicError>>()?
            .map_err(|err| RpcOpsError::RemoteRejected(err.to_string()))?;

        let response = R::try_from_response(response).map_err(RpcOpsError::from)?;

        Ok(response)
    }
}
