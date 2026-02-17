pub mod request;

use crate::{
    InternalError,
    dto::{
        error::Error,
        rpc::{AuthenticatedRequest, Response as DtoResponse},
    },
    ops::{
        OpsError,
        ic::call::{CallOps, CallResult},
        prelude::*,
        rpc::request::{Request, RequestOpsError, Response},
        runtime::env::EnvOps,
    },
    protocol,
};
use serde::de::DeserializeOwned;
use thiserror::Error as ThisError;

///
/// RpcOpsError
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] RequestOpsError),

    // Error is a wire-level contract.
    // It is preserved through the ops boundary.
    #[error("rpc rejected: {0}")]
    RemoteRejected(Error),
}

impl From<RpcOpsError> for InternalError {
    fn from(err: RpcOpsError) -> Self {
        match err {
            RpcOpsError::RemoteRejected(err) => Self::public(err),
            other @ RpcOpsError::RequestOps(_) => OpsError::from(other).into(),
        }
    }
}

///
/// Rpc
/// Typed RPC command binding a request variant to its response payload.
///

pub trait Rpc {
    type Response: CandidType + DeserializeOwned;

    fn into_request(self) -> Request;
    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError>;
}

///
/// RpcOps
///

pub struct RpcOps;

impl RpcOps {
    ///
    /// call_rpc_result
    ///
    /// Calls a method that returns `Result<T, Error>` and
    /// preserves `Error` at the ops boundary.
    ///
    pub async fn call_rpc_result<T>(
        pid: Principal,
        method: &str,
        arg: impl CandidType,
    ) -> Result<T, InternalError>
    where
        T: CandidType + DeserializeOwned,
    {
        let call: CallResult = CallOps::unbounded_wait(pid, method)
            .with_arg(arg)?
            .execute()
            .await?;

        let call_res: Result<T, Error> = call.candid::<Result<T, Error>>()?;

        let res = call_res.map_err(RpcOpsError::RemoteRejected)?;

        Ok(res)
    }

    ///
    /// execute_root_response_rpc
    ///
    /// Executes a protocol-level RPC via Request/Response.
    ///
    async fn execute_root_response_rpc<R: Rpc>(rpc: R) -> Result<R::Response, InternalError> {
        let root_pid = EnvOps::root_pid()?;

        let call: CallResult = CallOps::unbounded_wait(root_pid, protocol::CANIC_RESPONSE)
            .with_arg(rpc.into_request())?
            .execute()
            .await?;

        let call_res: Response = call
            .candid::<Result<Response, Error>>()?
            .map_err(RpcOpsError::RemoteRejected)?;

        let response = R::try_from_response(call_res)?;

        Ok(response)
    }

    ///
    /// call_authenticated_response
    ///
    /// Executes a protocol-level RPC via AuthenticatedRequest/Response.
    ///
    pub async fn call_authenticated_response(
        request: AuthenticatedRequest,
    ) -> Result<DtoResponse, InternalError> {
        let root_pid = EnvOps::root_pid()?;

        let call: CallResult =
            CallOps::unbounded_wait(root_pid, protocol::CANIC_RESPONSE_AUTHENTICATED)
                .with_arg(request)?
                .execute()
                .await?;

        let call_res: Result<DtoResponse, Error> = call.candid::<Result<DtoResponse, Error>>()?;

        Ok(call_res.map_err(RpcOpsError::RemoteRejected)?)
    }
}
