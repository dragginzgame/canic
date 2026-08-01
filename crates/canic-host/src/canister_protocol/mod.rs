//! Module: canister_protocol
//!
//! Responsibility: invoke typed Canic Candid methods through the maintained ICP CLI adapter.
//! Does not own: domain sequencing, endpoint authorization, or management-Canister effects.
//! Boundary: domain workflows supply exact Canister, method, arguments, and query/update intent.

use crate::icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response};
use candid::{CandidType, IDLValue, Principal};
use canic_core::dto::error::ErrorCode;
use serde::de::DeserializeOwned;
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";

#[derive(Debug, ThisError)]
pub enum CanisterProtocolError {
    #[error("failed to encode Candid arguments for {method} on Canister {canister}: {source}")]
    ArgumentEncoding {
        canister: Principal,
        method: &'static str,
        #[source]
        source: candid::Error,
    },

    #[error("failed to invoke {method} on Canister {canister}: {source}")]
    Invocation {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpCommandError,
    },

    #[error("invalid {method} response from Canister {canister}: {source}")]
    Response {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpJsonResponseError,
    },
}

impl CanisterProtocolError {
    pub(crate) fn is_rejected_with(&self, code: ErrorCode) -> bool {
        matches!(
            self,
            Self::Response {
                source: IcpJsonResponseError::Rejected(error),
                ..
            } if error.code == code
        )
    }
}

pub fn call_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    let output = icp
        .canister_call_arg_output_with_candid(
            &canister.to_text(),
            method,
            "()",
            Some(ICP_JSON_OUTPUT),
            None,
        )
        .map_err(|source| CanisterProtocolError::Invocation {
            canister,
            method,
            source,
        })?;
    decode_response(canister, method, &output)
}

pub fn call_with_arg<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
    input: &I,
    query: bool,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    let value = IDLValue::try_from_candid_type(input).map_err(|source| {
        CanisterProtocolError::ArgumentEncoding {
            canister,
            method,
            source,
        }
    })?;
    let args = format!("({value})");
    let output = if query {
        icp.canister_query_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )
    } else {
        icp.canister_call_arg_output_with_candid(
            &canister.to_text(),
            method,
            &args,
            Some(ICP_JSON_OUTPUT),
            None,
        )
    }
    .map_err(|source| CanisterProtocolError::Invocation {
        canister,
        method,
        source,
    })?;
    decode_response(canister, method, &output)
}

pub fn query_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    let output = icp
        .canister_query_output_with_candid(&canister.to_text(), method, Some(ICP_JSON_OUTPUT), None)
        .map_err(|source| CanisterProtocolError::Invocation {
            canister,
            method,
            source,
        })?;
    decode_response(canister, method, &output)
}

fn decode_response<O>(
    canister: Principal,
    method: &'static str,
    output: &str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    decode_json_result_response(output).map_err(|source| CanisterProtocolError::Response {
        canister,
        method,
        source,
    })
}
