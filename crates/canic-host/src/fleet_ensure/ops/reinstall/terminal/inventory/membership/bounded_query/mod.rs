//! Bounded authenticated observation using the caller's shared Agent.
//!
//! Owns transport and Candid budgets; callers retain protocol and authority checks.

#[cfg(test)]
mod tests;

use std::{future::Future, time::Duration};

use candid::{CandidType, Principal};
use canic_core::dto::error::Error;
use ic_agent::{Agent, AgentError};
use serde::de::DeserializeOwned;

///
/// QueryLimits
///
/// Per-query deadline and Candid decoding budgets.
///

pub(super) struct QueryLimits {
    pub timeout: Duration,
    pub response_bytes: usize,
}

///
/// QueryError
///
/// Transport failures before a caller interprets the observed response.
///

#[derive(Debug)]
pub(super) enum QueryError {
    Decode(candid::Error),

    Expired,

    Query(Box<AgentError>),

    Rejected(Box<Error>),
}

pub(super) async fn query<I: CandidType, O: CandidType + DeserializeOwned>(
    agent: &Agent,
    canister: Principal,
    method: &str,
    request: I,
    limits: QueryLimits,
) -> Result<O, QueryError> {
    let argument = candid::encode_one(request).map_err(QueryError::Decode)?;
    bounded_response(
        agent.query(&canister, method).with_arg(argument).call(),
        limits,
    )
    .await
}

async fn bounded_response<O: CandidType + DeserializeOwned>(
    response: impl Future<Output = Result<Vec<u8>, AgentError>>,
    limits: QueryLimits,
) -> Result<O, QueryError> {
    let bytes = tokio::time::timeout(limits.timeout, response)
        .await
        .map_err(|_| QueryError::Expired)?
        .map_err(|source| QueryError::Query(Box::new(source)))?;
    let mut config = candid::de::DecoderConfig::new();
    config.set_decoding_quota(limits.response_bytes * 64);
    config.set_skipping_quota(limits.response_bytes);
    let response: Result<O, Error> =
        candid::utils::decode_one_with_config(&bytes, &config).map_err(QueryError::Decode)?;
    response.map_err(|reason| QueryError::Rejected(Box::new(reason)))
}
