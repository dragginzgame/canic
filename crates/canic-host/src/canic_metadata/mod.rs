//! Module: canic_metadata
//!
//! Responsibility: query and decode the maintained Canic metadata endpoint.
//! Does not own: metadata production, ICP transport, or list rendering.
//! Boundary: projects the canonical metadata DTO into its reported Canic version.

#[cfg(test)]
mod tests;

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, IcpWireQueryError},
    protocol_binding::RegistryProtocolBinding,
};
use canic_contracts::dto::role::{RoleCapability, RoleOverviewResponse};
use canic_core::role_contract::RoleCapabilityKey;
use thiserror::Error as ThisError;

use canic_contracts::protocol::{CANIC_PUBLIC_OVERVIEW, public_overview};

///
/// CanicMetadataQueryError
///

#[derive(Debug, ThisError)]
pub enum CanicMetadataQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Wire(#[from] ic_wire::Error),

    #[error(transparent)]
    Argument(#[from] std::io::Error),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),

    #[error("role Overview conflicts with the exact preselected protocol binding: {0}")]
    Binding(String),
}

/// Query the role-owned Overview and return the reported Canic framework version.
pub fn query_canic_metadata_version(
    icp: &IcpCli,
    canister_id: &str,
    binding: &RegistryProtocolBinding,
) -> Result<String, CanicMetadataQueryError> {
    Ok(query_canic_overview(icp, canister_id, binding)?
        .metadata
        .canic_version)
}

/// Query the exact typed Overview and verify the preselected role/release profile.
pub(crate) fn query_canic_overview(
    icp: &IcpCli,
    canister_id: &str,
    binding: &RegistryProtocolBinding,
) -> Result<RoleOverviewResponse, CanicMetadataQueryError> {
    let result: Result<RoleOverviewResponse, canic_contracts::dto::error::Error> = icp
        .canister_query_wire(
            canister_id,
            CANIC_PUBLIC_OVERVIEW,
            public_overview::METHOD_ID,
            &(),
            public_overview::REQUEST_LIMITS,
            public_overview::RESPONSE_LIMITS,
        )?;
    let overview = result.map_err(IcpJsonResponseError::Rejected)?;
    verify_overview_binding(&overview, binding)?;
    Ok(overview)
}

#[cfg(test)]
fn parse_canic_metadata_version_response(output: &str) -> Result<String, CanicMetadataQueryError> {
    Ok(parse_canic_metadata_response(output)?
        .metadata
        .canic_version)
}

#[cfg(test)]
fn parse_canic_metadata_response(
    output: &str,
) -> Result<RoleOverviewResponse, CanicMetadataQueryError> {
    let result: Result<RoleOverviewResponse, canic_contracts::dto::error::Error> =
        crate::icp::decode_wire_response(
            output,
            public_overview::METHOD_ID,
            public_overview::RESPONSE_LIMITS,
        )?;
    result.map_err(|error| IcpJsonResponseError::Rejected(error).into())
}

impl From<IcpWireQueryError> for CanicMetadataQueryError {
    fn from(error: IcpWireQueryError) -> Self {
        match error {
            IcpWireQueryError::Icp(error) => Self::Icp(error),
            IcpWireQueryError::Wire(error) => Self::Wire(error),
            IcpWireQueryError::File(error) => Self::Argument(error),
            IcpWireQueryError::Response(error) => Self::Response(error),
        }
    }
}

fn verify_overview_binding(
    overview: &RoleOverviewResponse,
    selected: &RegistryProtocolBinding,
) -> Result<(), CanicMetadataQueryError> {
    let binding = selected;
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
        .filter(|capability| **capability != RoleCapabilityKey::LocalApplicationAuthorization)
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
        RoleCapability::ChildProvisioning => "ChildProvisioning",
        RoleCapability::CallerAuthority => "CallerAuthority",
        RoleCapability::DelegatedTokenIssuer => "DelegatedTokenIssuer",
        RoleCapability::DelegatedTokenVerifier => "DelegatedTokenVerifier",
        RoleCapability::FleetAdmissionProjection => "FleetAdmissionProjection",
        RoleCapability::FleetCoordinator => "FleetCoordinator",
        RoleCapability::Index => "Index",
        RoleCapability::ObservabilityDiagnostics => "ObservabilityDiagnostics",
        RoleCapability::ObservabilityHistory => "ObservabilityHistory",
        RoleCapability::ObservabilityLogs => "ObservabilityLogs",
        RoleCapability::ObservabilityMetrics => "ObservabilityMetrics",
        RoleCapability::RoleAttestationSigner => "RoleAttestationSigner",
        RoleCapability::RoleAttestationVerifier => "RoleAttestationVerifier",
        RoleCapability::Root => "Root",
        RoleCapability::RootControlPlane => "RootControlPlane",
        RoleCapability::RootDelegation => "RootDelegation",
        RoleCapability::Runtime => "Runtime",
        RoleCapability::Scaling => "Scaling",
        RoleCapability::Sharding => "Sharding",
        RoleCapability::WasmStore => "WasmStore",
    }
}
