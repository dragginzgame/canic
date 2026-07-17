//! Module: icp::response
//!
//! Responsibility: decode the canonical ICP CLI JSON response envelope.
//! Does not own: command execution, endpoint DTOs, or operator rendering.
//! Boundary: unwraps top-level `response_bytes` and decodes typed Candid values.

#[cfg(test)]
mod tests;

use candid::CandidType;
use canic_core::{
    cdk::utils::hash::{DecodeHexError, decode_hex},
    dto::error::Error as CanicError,
};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

#[derive(Deserialize)]
struct IcpJsonResponseEnvelope {
    response_bytes: Option<String>,
}

///
/// IcpJsonResponseError
///
/// Typed failure while decoding one ICP CLI JSON response envelope.
///

#[derive(Debug, ThisError)]
pub enum IcpJsonResponseError {
    #[error("ICP response_bytes Candid was invalid: {0}")]
    Candid(#[source] candid::Error),

    #[error("ICP response_bytes was invalid hexadecimal: {0}")]
    Hex(#[source] DecodeHexError),

    #[error("ICP response was invalid JSON: {0}")]
    Json(#[source] serde_json::Error),

    #[error("ICP JSON response is missing top-level string `response_bytes`")]
    MissingResponseBytes,

    #[error("canister rejected request: {0}")]
    Rejected(CanicError),
}

/// Decode a plain Candid value from the canonical ICP CLI JSON envelope.
pub fn decode_json_response<T>(output: &str) -> Result<T, IcpJsonResponseError>
where
    T: CandidType + DeserializeOwned,
{
    let bytes = response_bytes(output)?;
    candid::decode_one(&bytes).map_err(IcpJsonResponseError::Candid)
}

/// Decode a `Result<T, canic_core::dto::error::Error>` from the canonical envelope.
pub fn decode_json_result_response<T>(output: &str) -> Result<T, IcpJsonResponseError>
where
    T: CandidType + DeserializeOwned,
{
    let bytes = response_bytes(output)?;
    let response = candid::decode_one::<Result<T, CanicError>>(&bytes)
        .map_err(IcpJsonResponseError::Candid)?;
    response.map_err(IcpJsonResponseError::Rejected)
}

fn response_bytes(output: &str) -> Result<Vec<u8>, IcpJsonResponseError> {
    let envelope = serde_json::from_str::<IcpJsonResponseEnvelope>(output)
        .map_err(IcpJsonResponseError::Json)?;
    let response_bytes = envelope
        .response_bytes
        .ok_or(IcpJsonResponseError::MissingResponseBytes)?;
    decode_hex(&response_bytes).map_err(IcpJsonResponseError::Hex)
}
