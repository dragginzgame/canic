pub mod request;

use crate::{
    InternalError,
    dto::{
        auth::{RoleAttestationRequest, SignedRoleAttestation},
        error::Error,
        rpc::RootRequestMetadata,
    },
    ops::{
        OpsError,
        config::ConfigOps,
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        prelude::*,
        rpc::request::{Request, RequestOpsError, Response},
        runtime::env::EnvOps,
    },
    protocol,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
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

const DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_SECONDS: u64 = 300;
static ROOT_ATTESTATION_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

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
        let request = rpc.into_request();
        let attestation = Self::request_root_response_attestation(root_pid).await?;
        let call_res = Self::call_root_response_attested(root_pid, request, attestation).await?;

        let response = R::try_from_response(call_res)?;

        Ok(response)
    }

    async fn request_root_response_attestation(
        root_pid: Principal,
    ) -> Result<SignedRoleAttestation, InternalError> {
        let self_pid = IcOps::canister_self();
        let role = EnvOps::canister_role()?;
        let cfg = ConfigOps::role_attestation_config()?;
        let epoch = cfg
            .min_accepted_epoch_by_role
            .get(role.as_str())
            .copied()
            .unwrap_or(0);

        let request = RoleAttestationRequest {
            subject: self_pid,
            role,
            subnet_id: None,
            audience: Some(root_pid),
            ttl_secs: cfg.max_ttl_secs,
            epoch,
            metadata: Some(new_root_attestation_request_metadata()),
        };

        Self::call_rpc_result(root_pid, protocol::CANIC_REQUEST_ROLE_ATTESTATION, request).await
    }

    async fn call_root_response_attested(
        root_pid: Principal,
        request: Request,
        attestation: SignedRoleAttestation,
    ) -> Result<Response, InternalError> {
        let call: CallResult = CallOps::unbounded_wait(root_pid, protocol::CANIC_RESPONSE_ATTESTED)
            .with_args((request, attestation, 0u64))?
            .execute()
            .await?;

        call.candid::<Result<Response, Error>>()?
            .map_err(|err| RpcOpsError::RemoteRejected(err).into())
    }
}

fn new_root_attestation_request_metadata() -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: generate_root_attestation_request_id(),
        ttl_seconds: DEFAULT_ROOT_ATTESTATION_REQUEST_TTL_SECONDS,
    }
}

fn generate_root_attestation_request_id() -> [u8; 32] {
    if let Ok(bytes) = crate::utils::rand::random_bytes(32)
        && bytes.len() == 32
    {
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        return out;
    }

    let nonce = ROOT_ATTESTATION_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::msg_caller();
    let canister = IcOps::canister_self();

    let mut hasher = Sha256::new();
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}
