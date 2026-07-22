//! Module: replica_query::cbor
//!
//! Responsibility: encode and decode the IC replica CBOR wire boundary.
//! Does not own: HTTP transport, Candid reply payloads, or replica targeting.
//! Boundary: codec-specific values and errors do not escape this module.

use canic_core::cdk::utils::hash::hex_bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

/// Codec-neutral CBOR boundary failure.
#[derive(Debug, ThisError)]
#[error("{0}")]
pub(super) struct CborError(String);

/// Decoded outcome of one replica query response.
pub(super) enum QueryOutcome {
    Replied(Vec<u8>),
    Rejected { code: u64, message: String },
}

pub(super) fn encode_anonymous_query(
    canister_id: &[u8],
    method_name: &str,
    arg: &[u8],
    ingress_expiry: u64,
) -> Result<Vec<u8>, CborError> {
    let sender = candid::Principal::anonymous();
    let envelope = QueryEnvelope {
        content: QueryContent {
            request_type: "query",
            canister_id,
            method_name,
            arg,
            sender: sender.as_slice(),
            ingress_expiry,
        },
    };
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&envelope, &mut bytes).map_err(cbor_error)?;
    Ok(bytes)
}

pub(super) fn decode_query_response(bytes: &[u8]) -> Result<QueryOutcome, CborError> {
    let response = ciborium::de::from_reader::<QueryResponse, _>(bytes).map_err(cbor_error)?;
    match response.status {
        QueryResponseStatus::Replied => response
            .reply
            .map(|reply| QueryOutcome::Replied(reply.arg))
            .ok_or_else(|| CborError("missing query reply".to_string())),
        QueryResponseStatus::Rejected => Ok(QueryOutcome::Rejected {
            code: response.reject_code.unwrap_or_default(),
            message: response.reject_message.unwrap_or_default(),
        }),
    }
}

pub(super) fn decode_status_root_key(bytes: &[u8]) -> Result<Option<String>, CborError> {
    let value = ciborium::de::from_reader::<ciborium::Value, _>(bytes).map_err(cbor_error)?;
    if matches!(
        value,
        ciborium::Value::Bytes(_)
            | ciborium::Value::Text(_)
            | ciborium::Value::Array(_)
            | ciborium::Value::Map(_)
    ) {
        return Ok(root_key_from_value(&value));
    }
    Err(CborError(
        "unsupported replica status CBOR shape".to_string(),
    ))
}

fn root_key_from_value(value: &ciborium::Value) -> Option<String> {
    match value {
        ciborium::Value::Bytes(bytes) => (!bytes.is_empty()).then(|| hex_bytes(bytes)),
        ciborium::Value::Text(text) => nonempty_text(text),
        ciborium::Value::Array(values) => values.iter().find_map(root_key_from_value),
        ciborium::Value::Map(map) => map
            .iter()
            .find_map(|(key, value)| match key {
                ciborium::Value::Text(key) if key == "root_key" => root_key_from_value(value),
                _ => None,
            })
            .or_else(|| map.iter().find_map(|(_, value)| root_key_from_value(value))),
        _ => None,
    }
}

fn nonempty_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn cbor_error(error: impl std::fmt::Display) -> CborError {
    CborError(error.to_string())
}

#[derive(Serialize)]
struct QueryEnvelope<'a> {
    content: QueryContent<'a>,
}

#[derive(Serialize)]
struct QueryContent<'a> {
    request_type: &'static str,
    #[serde(with = "serde_bytes")]
    canister_id: &'a [u8],
    method_name: &'a str,
    #[serde(with = "serde_bytes")]
    arg: &'a [u8],
    #[serde(with = "serde_bytes")]
    sender: &'a [u8],
    ingress_expiry: u64,
}

#[derive(Deserialize)]
struct QueryResponse {
    status: QueryResponseStatus,
    reply: Option<QueryReply>,
    reject_code: Option<u64>,
    reject_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum QueryResponseStatus {
    Rejected,
    Replied,
}

#[derive(Deserialize)]
struct QueryReply {
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

#[cfg(test)]
mod tests;
