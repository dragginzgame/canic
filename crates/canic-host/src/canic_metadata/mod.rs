use crate::{
    icp::{IcpCli, IcpCommandError},
    response_parse::{find_string_field, parse_candid_text_field, response_candid},
};
use canic_core::protocol;
use std::path::Path;

const ICP_JSON_OUTPUT: &str = "json";

/// Query `canic_metadata` and return the reported Canic framework version.
pub fn query_canic_metadata_version(
    icp: &IcpCli,
    canister_id: &str,
    candid_path: Option<&Path>,
) -> Result<Option<String>, IcpCommandError> {
    let output = icp.canister_query_output_with_candid(
        canister_id,
        protocol::CANIC_METADATA,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    Ok(parse_canic_metadata_version_response(&output))
}

/// Parse a Canic framework version from `canic_metadata` command output.
#[must_use]
pub fn parse_canic_metadata_version_response(output: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| {
            find_string_field(&value, "canic_version").or_else(|| {
                response_candid(&value)
                    .and_then(|candid| parse_candid_text_field(candid, "canic_version"))
            })
        })
        .or_else(|| parse_candid_text_field(output, "canic_version"))
}

#[cfg(test)]
mod tests;
