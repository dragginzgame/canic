//! Module: protocol_binding
//!
//! Responsibility: validate exact host-side protocol bindings before Canister transport.
//! Does not own: Fleet discovery, artifact production, or live Overview negotiation.
//! Boundary: immutable release/Directory evidence must reproduce the local Candid profile.

use crate::{
    icp::existing_local_canister_candid_path, registry::RegistryEntry,
    release_set::CanicInfrastructureArtifactEntry,
};
use canic_core::{
    ids::CanisterRole,
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey, derive_protocol_profile_hashes},
};
use std::{collections::BTreeSet, fs, io, path::PathBuf};
use thiserror::Error as ThisError;

/// Complete immutable protocol identity selected before one role call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryProtocolBinding {
    pub release_identity: String,
    pub role: CanisterRole,
    pub capabilities: BTreeSet<RoleCapabilityKey>,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

/// Validated binding plus the exact local Candid sidecar admitted for transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedProtocolBinding {
    pub(crate) binding: RegistryProtocolBinding,
    pub(crate) candid_path: PathBuf,
}

impl ResolvedProtocolBinding {
    /// Return the exact immutable identity verified by the resolver.
    #[must_use]
    pub const fn binding(&self) -> &RegistryProtocolBinding {
        &self.binding
    }

    /// Return the exact verified local Candid sidecar admitted for transport.
    #[must_use]
    pub fn candid_path(&self) -> &std::path::Path {
        &self.candid_path
    }
}

/// Failure to select an exact protocol profile before transport.
#[derive(Debug, ThisError)]
pub enum ProtocolBindingError {
    #[error("Canister {canister} has no exact protocol binding in protected registry metadata")]
    MissingBinding { canister: String },

    #[error("Canister {canister} has no exact role in protected registry metadata")]
    MissingRole { canister: String },

    #[error(
        "Canister {canister} registry role {registry_role} conflicts with protocol role {protocol_role}"
    )]
    RoleMismatch {
        canister: String,
        registry_role: String,
        protocol_role: String,
    },

    #[error("Canister {canister} protocol release identity is empty")]
    MissingReleaseIdentity { canister: String },

    #[error("Canister {canister} has no local Candid sidecar for exact role {role}")]
    MissingCandid { canister: String, role: String },

    #[error("failed to read exact Candid sidecar {path}: {source}")]
    ReadCandid {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Canister {canister} local Candid hash conflicts with protected registry metadata")]
    CandidHashMismatch { canister: String },

    #[error(
        "Canister {canister} protocol-profile digest conflicts with protected registry metadata"
    )]
    ProfileDigestMismatch { canister: String },

    #[error(
        "infrastructure artifact role {artifact_role} conflicts with protocol role {protocol_role}"
    )]
    InfrastructureRoleMismatch {
        artifact_role: String,
        protocol_role: String,
    },
}

/// Select and verify one exact registry-owned local binding before transport.
pub fn resolve_registry_protocol_binding(
    icp_root: &std::path::Path,
    artifact_environment: &str,
    entry: &RegistryEntry,
) -> Result<ResolvedProtocolBinding, ProtocolBindingError> {
    let binding =
        entry
            .protocol_binding
            .clone()
            .ok_or_else(|| ProtocolBindingError::MissingBinding {
                canister: entry.pid.clone(),
            })?;
    let registry_role = entry
        .role
        .as_deref()
        .ok_or_else(|| ProtocolBindingError::MissingRole {
            canister: entry.pid.clone(),
        })?;
    if binding.release_identity.trim().is_empty() {
        return Err(ProtocolBindingError::MissingReleaseIdentity {
            canister: entry.pid.clone(),
        });
    }
    if registry_role != binding.role.as_str() {
        return Err(ProtocolBindingError::RoleMismatch {
            canister: entry.pid.clone(),
            registry_role: registry_role.to_string(),
            protocol_role: binding.role.to_string(),
        });
    }
    resolve_local_protocol_binding(icp_root, artifact_environment, &entry.pid, binding)
}

/// Select and verify one exact immutable infrastructure-artifact binding before transport.
pub fn resolve_infrastructure_protocol_binding(
    icp_root: &std::path::Path,
    artifact_environment: &str,
    artifact: &CanicInfrastructureArtifactEntry,
) -> Result<ResolvedProtocolBinding, ProtocolBindingError> {
    if artifact.protocol_role.as_str() != artifact.role.as_str() {
        return Err(ProtocolBindingError::InfrastructureRoleMismatch {
            artifact_role: artifact.role.as_str().to_string(),
            protocol_role: artifact.protocol_role.to_string(),
        });
    }
    resolve_local_protocol_binding(
        icp_root,
        artifact_environment,
        artifact.role.as_str(),
        RegistryProtocolBinding {
            release_identity: artifact.protocol_release_identity.clone(),
            role: artifact.protocol_role.clone(),
            capabilities: artifact.protocol_capabilities.clone(),
            candid_sha256: artifact.candid_sha256,
            protocol_profile_digest: artifact.protocol_profile_digest,
        },
    )
}

fn resolve_local_protocol_binding(
    icp_root: &std::path::Path,
    artifact_environment: &str,
    target: &str,
    binding: RegistryProtocolBinding,
) -> Result<ResolvedProtocolBinding, ProtocolBindingError> {
    if binding.release_identity.trim().is_empty() {
        return Err(ProtocolBindingError::MissingReleaseIdentity {
            canister: target.to_string(),
        });
    }
    let candid_path =
        existing_local_canister_candid_path(icp_root, artifact_environment, binding.role.as_str())
            .ok_or_else(|| ProtocolBindingError::MissingCandid {
                canister: target.to_string(),
                role: binding.role.to_string(),
            })?;
    let candid = fs::read(&candid_path).map_err(|source| ProtocolBindingError::ReadCandid {
        path: candid_path.clone(),
        source,
    })?;
    let observed = derive_protocol_profile_hashes(
        &binding.release_identity,
        &binding.role,
        &binding.capabilities,
        &candid,
    );
    if observed.candid_sha256 != binding.candid_sha256 {
        return Err(ProtocolBindingError::CandidHashMismatch {
            canister: target.to_string(),
        });
    }
    if observed.protocol_profile_digest != binding.protocol_profile_digest {
        return Err(ProtocolBindingError::ProfileDigestMismatch {
            canister: target.to_string(),
        });
    }
    Ok(ResolvedProtocolBinding {
        binding,
        candid_path,
    })
}

#[cfg(test)]
mod tests;
