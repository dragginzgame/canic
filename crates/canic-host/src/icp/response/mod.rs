//! Module: icp::response
//!
//! Responsibility: decode the canonical ICP CLI JSON response envelope.
//! Does not own: command execution, endpoint DTOs, or operator rendering.
//! Boundary: unwraps top-level `response_bytes` and decodes typed Candid values.

#[cfg(test)]
mod tests;

use candid::CandidType;
use canic_core::dto::error::Error as CanicError;
use ic_host_tools::response::{ResponseError, ResponseFormat, ResponseLimits, decode};
use serde::de::DeserializeOwned;
use thiserror::Error as ThisError;

///
/// IcpJsonResponseError
///
/// Typed failure while decoding one ICP CLI JSON response envelope.
///

#[derive(Debug, ThisError)]
pub enum IcpJsonResponseError {
    #[error("ICP response_bytes Candid was invalid: {0}")]
    Candid(#[source] candid::Error),

    #[error("ICP response envelope was invalid: {0}")]
    Envelope(#[from] ResponseError),

    #[error(
        "canister rejected request: {diagnostic}",
        diagnostic = crate::diagnostics::render_diagnostic(.0.code())
    )]
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

pub fn response_bytes(output: &str) -> Result<Vec<u8>, IcpJsonResponseError> {
    // Capture admission belongs to the command owner. Hex cannot produce more
    // than half the captured input; the codec owns validation and allocation.
    decode(
        output.as_bytes(),
        ResponseFormat::Json,
        ResponseLimits {
            input_bytes: output.len(),
            decoded_bytes: output.len() / 2,
        },
    )
    .map_err(IcpJsonResponseError::Envelope)
}
