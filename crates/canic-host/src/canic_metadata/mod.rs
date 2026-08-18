//! Module: canic_metadata
//!
//! Responsibility: query and decode the maintained Canic metadata endpoint.
//! Does not own: metadata production, ICP transport, or list rendering.
//! Boundary: projects the canonical metadata DTO into its reported Canic version.

#[cfg(test)]
mod tests;

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{CandidType, Deserialize};
use canic_core::{
    dto::role::{RoleCapability, RoleOverviewResponse},
    protocol,
};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    Overview(RoleOverviewResponse),
}

///
/// CanicMetadataQueryError
///

#[derive(Debug, ThisError)]
pub enum CanicMetadataQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),

    #[error("role Overview conflicts with the exact preselected protocol binding: {0}")]
    Binding(String),
}

/// Query the role-owned Overview and return the reported Canic framework version.
pub fn query_canic_metadata_version(
    icp: &IcpCli,
    canister_id: &str,
    binding: &ResolvedProtocolBinding,
) -> Result<String, CanicMetadataQueryError> {
    let output = icp.canister_query_arg_output_with_candid(
        canister_id,
        protocol::CANIC_STATUS,
        "(variant { Overview })",
        Some(ICP_JSON_OUTPUT),
        Some(binding.candid_path.as_path()),
    )?;
    let overview = parse_canic_metadata_response(&output)?;
    verify_overview_binding(&overview, binding)?;
    Ok(overview.metadata.canic_version)
}

#[cfg(test)]
fn parse_canic_metadata_version_response(output: &str) -> Result<String, IcpJsonResponseError> {
    Ok(parse_canic_metadata_response(output)?
        .metadata
        .canic_version)
}

fn parse_canic_metadata_response(
    output: &str,
) -> Result<RoleOverviewResponse, IcpJsonResponseError> {
    let response = decode_json_result_response::<RoleStatusResponse>(output)?;
    let RoleStatusResponse::Overview(overview) = response;
    Ok(overview)
}

fn verify_overview_binding(
    overview: &RoleOverviewResponse,
    selected: &ResolvedProtocolBinding,
) -> Result<(), CanicMetadataQueryError> {
    let binding = &selected.binding;
    if overview.role != binding.role {
        return Err(CanicMetadataQueryError::Binding(
            "role mismatch".to_string(),
        ));
    }
    if overview.metadata.canic_version != binding.release_identity {
        return Err(CanicMetadataQueryError::Binding(
            "release identity mismatch".to_string(),
        ));
    }
    let capabilities = overview
        .capabilities
        .iter()
        .copied()
        .map(role_capability_name)
        .collect::<Vec<_>>();
    let expected = binding
        .capabilities
        .iter()
        .map(|capability| capability.manifest_name())
        .collect::<Vec<_>>();
    if capabilities != expected {
        return Err(CanicMetadataQueryError::Binding(
            "ordered capability mismatch".to_string(),
        ));
    }
    if overview.protocol_profile_digest != binding.protocol_profile_digest.into_bytes() {
        return Err(CanicMetadataQueryError::Binding(
            "profile digest mismatch".to_string(),
        ));
    }
    Ok(())
}

const fn role_capability_name(capability: RoleCapability) -> &'static str {
    match capability {
        RoleCapability::AutomaticTopup => "AutomaticTopup",
        RoleCapability::DelegatedTokenIssuer => "DelegatedTokenIssuer",
        RoleCapability::DelegatedTokenVerifier => "DelegatedTokenVerifier",
        RoleCapability::FleetCoordinator => "FleetCoordinator",
        RoleCapability::Icrc21 => "Icrc21",
        RoleCapability::Index => "Index",
        RoleCapability::RoleAttestationSigner => "RoleAttestationSigner",
        RoleCapability::RoleAttestationVerifier => "RoleAttestationVerifier",
        RoleCapability::Root => "Root",
        RoleCapability::RootControlPlane => "RootControlPlane",
        RoleCapability::Runtime => "Runtime",
        RoleCapability::Scaling => "Scaling",
        RoleCapability::Sharding => "Sharding",
        RoleCapability::WasmStore => "WasmStore",
    }
}
