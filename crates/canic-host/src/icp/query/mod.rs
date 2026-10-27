//! Module: icp::query
//!
//! Responsibility: bounded authenticated reads with typed transient retries.
//! Boundary: this transport exposes queries only; identity and network remain ICP-owned.

#[cfg(test)]
mod tests;

use crate::icp::{IcpCli, IcpManagementCallError};
use ic_platform::{
    agent::{Agent, AgentError, TransportError},
    principal::ExternalPrincipal as Principal,
};
use serde::de::DeserializeOwned;
use std::{future::Future, sync::Arc, time::Duration};
use thiserror::Error;

// Bound the transport envelope, not the logical contents of application storage.
const RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const ATTEMPTS: usize = 3;
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

///
/// IcpQueryError
///
/// Transport-owned authenticated read failure. Application rejections and decode
/// failures never authorize a retry.
///
#[derive(Debug, Error)]
pub enum IcpQueryError {
    #[error("authenticated query failed: {0}")]
    Agent(#[source] Box<AgentError>),

    #[error("query identity/network resolution failed: {0}")]
    Authority(#[source] Box<IcpManagementCallError>),

    #[error("authenticated query exceeded its bounded read deadline")]
    Deadline,

    #[error("query Wire codec failed: {0}")]
    Wire(#[from] ic_wire::Error),

    #[error("query runtime failed: {0}")]
    Runtime(#[source] std::io::Error),

    #[error("query HTTP client setup failed: {0}")]
    Client(#[source] reqwest::Error),
}

/// Connection pool and one IO worker shared by a command's cloned query contexts.
/// Contains no signer, network authority, Agent or query response cache.
#[derive(Debug)]
pub(super) struct QueryTransport {
    client: reqwest::Client,
    runtime: Option<tokio::runtime::Runtime>,
}

impl QueryTransport {
    fn new() -> Result<Self, IcpQueryError> {
        // Match ic-agent's native client defaults; query deadlines remain tighter.
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(360))
            .build()
            .map_err(IcpQueryError::Client)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("canic-query")
            .enable_all()
            .build()
            .map_err(IcpQueryError::Runtime)?;
        Ok(Self {
            client,
            runtime: Some(runtime),
        })
    }

    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime
            .as_ref()
            .expect("live query transport")
            .block_on(future)
    }
}

impl Drop for QueryTransport {
    fn drop(&mut self) {
        // IcpCli may be owned by an async caller. No query can still borrow this
        // final Arc; shut down idle IO without blocking that caller's runtime.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

impl IcpCli {
    fn query_transport(&self) -> Result<Arc<QueryTransport>, IcpQueryError> {
        let mut retained = self.query_transport.lock().expect("query transport lock");
        if let Some(transport) = retained.as_ref() {
            return Ok(Arc::clone(transport));
        }
        let transport = Arc::new(QueryTransport::new()?);
        *retained = Some(Arc::clone(&transport));
        drop(retained);
        Ok(transport)
    }

    /// Query a native Wire contract with the same authenticated bounded read owner.
    pub(crate) fn query_wire_readonly<I: serde::Serialize, O: DeserializeOwned>(
        &self,
        canister: Principal,
        method: &str,
        method_id: u64,
        input: &I,
        request_limits: ic_wire::Limits,
        response_limits: ic_wire::Limits,
    ) -> Result<O, IcpQueryError> {
        self.measure_request(
            crate::icp::IcpRequestKind::AgentQuery,
            Some(&canister.to_text()),
            Some(method),
            || {
                let argument = ic_wire::native::encode_request(method_id, input, request_limits)?;
                let bytes = self.query_readonly_bytes(canister, method, &argument)?;
                ic_wire::native::decode_response(method_id, &bytes, response_limits)
                    .map_err(IcpQueryError::Wire)
            },
        )
    }

    fn query_readonly_bytes(
        &self,
        canister: Principal,
        method: &str,
        argument: &[u8],
    ) -> Result<Vec<u8>, IcpQueryError> {
        let transport = self.query_transport()?;
        let agent = self
            .build_authenticated_agent(
                Agent::builder()
                    .with_http_client(transport.client.clone())
                    .with_max_response_body_size(RESPONSE_BYTES),
            )
            .map_err(|error| IcpQueryError::Authority(Box::new(error)))?;
        let bytes = query_bytes(self, &agent, canister, method, argument)?;

        Ok(bytes)
    }
}

fn query_bytes(
    icp: &IcpCli,
    agent: &Agent,
    canister: Principal,
    method: &str,
    argument: &[u8],
) -> Result<Vec<u8>, IcpQueryError> {
    icp.query_transport()?.block_on(async {
        tokio::time::timeout(
            TOTAL_TIMEOUT,
            retry_query(|| async {
                icp.record_remote_call();
                tokio::time::timeout(
                    ATTEMPT_TIMEOUT,
                    agent.query(&canister, method).with_arg(argument).call(),
                )
                .await
                .map_err(|_| IcpQueryError::Deadline)?
                .map_err(|error| IcpQueryError::Agent(Box::new(error)))
            }),
        )
        .await
        .map_err(|_| IcpQueryError::Deadline)?
    })
}

async fn retry_query<T, F, Fut>(mut query: F) -> Result<T, IcpQueryError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, IcpQueryError>>,
{
    for attempt in 0..ATTEMPTS {
        match query().await {
            Err(error) if attempt + 1 < ATTEMPTS && transient(&error) => {
                tokio::time::sleep(Duration::from_millis(250 << attempt)).await;
            }
            result => return result,
        }
    }
    unreachable!("the last attempt always returns")
}

fn transient(error: &IcpQueryError) -> bool {
    match error {
        IcpQueryError::Deadline => true,
        IcpQueryError::Agent(error) => match error.as_ref() {
            AgentError::TimeoutWaitingForResponse() => true,
            AgentError::HttpError(error) => matches!(error.status, 408 | 429 | 502 | 503 | 504),
            AgentError::TransportError(TransportError::Reqwest(error)) => {
                error.is_timeout() || error.is_connect() || error.is_body()
            }
            _ => false,
        },
        _ => false,
    }
}
