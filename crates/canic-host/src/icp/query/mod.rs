//! Module: icp::query
//!
//! Responsibility: bounded authenticated reads with typed transient retries.
//! Boundary: this transport exposes queries only; identity and network remain ICP-owned.

#[cfg(test)]
mod tests;

use crate::icp::{IcpCli, IcpManagementCallError};
use candid::{CandidType, Principal};
use ic_agent::{Agent, AgentError, agent::agent_error::TransportError};
use serde::de::DeserializeOwned;
use std::{future::Future, time::Duration};
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

    #[error("query response decoding failed: {0}")]
    Decode(#[source] candid::Error),

    #[error("query argument encoding failed: {0}")]
    Encode(#[source] candid::Error),

    #[error("query runtime failed: {0}")]
    Runtime(#[source] std::io::Error),
}

impl IcpCli {
    /// Query one immutable request with at most three logical attempts in 30 seconds
    /// after local identity/network resolution. Agent-internal HTTP retries and
    /// certificate reads share that deadline but are not separate logical attempts.
    /// The same verified signer, network, target, method and argument bind every attempt.
    pub(crate) fn query_candid_readonly<I: CandidType, O: CandidType + DeserializeOwned>(
        &self,
        canister: Principal,
        method: &str,
        input: &I,
    ) -> Result<O, IcpQueryError> {
        self.measure_request(
            crate::icp::IcpRequestKind::AgentQuery,
            Some(&canister.to_text()),
            Some(method),
            || {
                let argument = candid::encode_one(input).map_err(IcpQueryError::Encode)?;
                let agent = self
                    .authenticated_agent_with_response_limit(RESPONSE_BYTES)
                    .map_err(|error| IcpQueryError::Authority(Box::new(error)))?;
                let bytes = query_bytes(self, &agent, canister, method, &argument)?;
                let mut config = candid::de::DecoderConfig::new();
                config.set_decoding_quota(RESPONSE_BYTES * 64);
                config.set_skipping_quota(RESPONSE_BYTES);
                candid::utils::decode_one_with_config(&bytes, &config)
                    .map_err(IcpQueryError::Decode)
            },
        )
    }
}

fn query_bytes(
    icp: &IcpCli,
    agent: &Agent,
    canister: Principal,
    method: &str,
    argument: &[u8],
) -> Result<Vec<u8>, IcpQueryError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(IcpQueryError::Runtime)?;
    runtime.block_on(async {
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
