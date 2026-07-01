//! Module: infra::ic::http
//!
//! Responsibility: perform raw IC HTTP outcalls.
//! Does not own: HTTP policy, response validation, metrics, or workflow retries.
//! Boundary: ops calls this adapter with already-approved HTTP request arguments.

use crate::{
    cdk::{
        api,
        candid::{CandidType, Func, Nat, Principal},
    },
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};
use serde::Deserialize;
use thiserror::Error as ThisError;

///
/// InfraHttpRequestArgs
///
/// Raw management-canister HTTP outcall payload.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraHttpRequestArgs {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: InfraHttpMethod,
    pub headers: Vec<InfraHttpHeader>,
    pub body: Option<Vec<u8>>,
    pub transform: Option<InfraTransformContext>,
    pub is_replicated: Option<bool>,
}

///
/// InfraHttpRequestResult
///
/// Raw management-canister HTTP outcall result.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraHttpRequestResult {
    pub status: Nat,
    pub headers: Vec<InfraHttpHeader>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

///
/// InfraHttpMethod
///
/// Raw management-canister HTTP method selector.
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub enum InfraHttpMethod {
    #[default]
    #[serde(rename = "get")]
    Get,
    #[serde(rename = "post")]
    Post,
    #[serde(rename = "head")]
    Head,
}

///
/// InfraHttpHeader
///
/// Raw management-canister HTTP header.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraHttpHeader {
    pub name: String,
    pub value: String,
}

///
/// InfraTransformContext
///
/// Optional management-canister HTTP transform context.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraTransformContext {
    pub function: Func,
    #[serde(with = "serde_bytes")]
    pub context: Vec<u8>,
}

///
/// HttpInfraError
///
/// Raw HTTP outcall failure surfaced by IC infra.
/// Owned by HTTP infra and converted into `InfraError`.
///

#[derive(Debug, ThisError)]
pub enum HttpInfraError {
    #[error(transparent)]
    RequestFailed(#[from] crate::cdk::call::Error),
}

impl From<HttpInfraError> for InfraError {
    fn from(err: HttpInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}

///
/// HttpInfra
///
/// Raw IC HTTP outcall facade.
/// Owned by IC infra and used by ops HTTP adapters.
///

pub struct HttpInfra;

impl HttpInfra {
    /// Raw IC HTTP request passthrough.
    ///
    /// No metrics, no validation, no interpretation.
    pub async fn http_request_raw(
        args: &InfraHttpRequestArgs,
    ) -> Result<InfraHttpRequestResult, InfraError> {
        let cycles = cost_http_request(args);
        let response = Call::unbounded_wait(Principal::management_canister(), "http_request")
            .with_arg(args.clone())?
            .with_cycles(cycles)
            .execute()
            .await?;
        response.candid()
    }
}

fn cost_http_request(args: &InfraHttpRequestArgs) -> u128 {
    let request_size = (args.url.len()
        + args
            .headers
            .iter()
            .map(|header| header.name.len() + header.value.len())
            .sum::<usize>()
        + args.body.as_ref().map_or(0, Vec::len)
        + args.transform.as_ref().map_or(0, |transform| {
            transform.context.len() + transform.function.method.len()
        })) as u64;
    let max_response_bytes = args.max_response_bytes.unwrap_or(2_000_000);
    api::cost_http_request(request_size, max_response_bytes)
}
