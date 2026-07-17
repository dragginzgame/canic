//! Module: canic_metadata
//!
//! Responsibility: query and decode the maintained Canic metadata endpoint.
//! Does not own: metadata production, ICP transport, or list rendering.
//! Boundary: projects the canonical metadata DTO into its reported Canic version.

#[cfg(test)]
mod tests;

use crate::icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_response};
use std::path::Path;

use canic_core::{dto::metadata::CanicMetadataResponse, protocol};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";

///
/// CanicMetadataQueryError
///

#[derive(Debug, ThisError)]
pub enum CanicMetadataQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

/// Query `canic_metadata` and return the reported Canic framework version.
pub fn query_canic_metadata_version(
    icp: &IcpCli,
    canister_id: &str,
    candid_path: Option<&Path>,
) -> Result<String, CanicMetadataQueryError> {
    let output = icp.canister_query_output_with_candid(
        canister_id,
        protocol::CANIC_METADATA,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    parse_canic_metadata_version_response(&output).map_err(Into::into)
}

/// Parse a Canic framework version from `canic_metadata` command output.
pub fn parse_canic_metadata_version_response(output: &str) -> Result<String, IcpJsonResponseError> {
    let metadata = decode_json_response::<CanicMetadataResponse>(output)?;
    Ok(metadata.canic_version)
}
