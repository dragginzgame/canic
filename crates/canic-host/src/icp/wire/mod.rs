//! Typed Wire queries and updates through ICP’s selected identity, without a Candid sidecar.

#[cfg(all(test, unix))]
mod tests;

use super::{
    IcpCli, IcpCommandError, IcpJsonResponseError, cleanup_binary_argument_file,
    write_binary_argument_file,
};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

/// Transport and framing failures; application errors remain in the typed response.
#[derive(Debug, Error)]
pub enum IcpWireQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),
    #[error(transparent)]
    Wire(#[from] ic_wire::Error),
    #[error(transparent)]
    File(#[from] std::io::Error),
    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

impl IcpCli {
    /// Preserve the CLI caller's identity and one-query process semantics.
    pub fn canister_query_wire<I: Serialize, O: DeserializeOwned>(
        &self,
        canister: &str,
        method: &str,
        method_id: u64,
        input: &I,
        request_limits: ic_wire::Limits,
        response_limits: ic_wire::Limits,
    ) -> Result<O, IcpWireQueryError> {
        let bytes = ic_wire::native::encode_request(method_id, input, request_limits)?;
        let argument = write_binary_argument_file(&bytes)?;
        let output = self.canister_query_binary_args_output_with_candid(
            canister,
            method,
            &argument,
            Some("hex"),
            None,
        );
        cleanup_binary_argument_file(&argument);
        decode_wire_response(&output?, method_id, response_limits)
    }
    /// Preserve the CLI caller's identity and one-update process semantics.
    pub fn canister_update_wire<I: Serialize, O: DeserializeOwned>(
        &self,
        canister: &str,
        method: &str,
        method_id: u64,
        input: &I,
        request_limits: ic_wire::Limits,
        response_limits: ic_wire::Limits,
    ) -> Result<O, IcpWireQueryError> {
        let bytes = ic_wire::native::encode_request(method_id, input, request_limits)?;
        let argument = write_binary_argument_file(&bytes)?;
        let output = self.canister_call_binary_args_output_with_candid(
            canister,
            method,
            &argument,
            Some("hex"),
            None,
        );
        cleanup_binary_argument_file(&argument);
        decode_wire_response(&output?, method_id, response_limits)
    }
}

pub fn decode_wire_response<O: DeserializeOwned>(
    output: &str,
    method_id: u64,
    limits: ic_wire::Limits,
) -> Result<O, IcpWireQueryError> {
    let bytes = ic_host_tools::response::decode(
        output.as_bytes(),
        ic_host_tools::response::ResponseFormat::Hex,
        ic_host_tools::response::ResponseLimits {
            input_bytes: limits.bytes.saturating_mul(2).saturating_add(1024),
            decoded_bytes: limits.bytes,
        },
    )
    .map_err(IcpJsonResponseError::Envelope)?;
    Ok(ic_wire::native::decode_response(method_id, &bytes, limits)?)
}
