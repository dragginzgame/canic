//! Module: install_root::operations::binding
//!
//! Responsibility: resolve one immutable infrastructure protocol profile before transport.
//! Does not own: artifact production, installation sequencing, or Overview verification.
//! Boundary: the finalized release-build manifest and exact local Candid must agree.

use super::super::icp_context::InstallIcpContext;
use crate::{
    protocol_binding::{
        ProtocolBindingError, ResolvedProtocolBinding, resolve_infrastructure_protocol_binding,
    },
    release_set::{CanicInfrastructureRole, PersistedCanicInfrastructureArtifactManifest},
};

pub(in crate::install_root) fn resolve_install_protocol_binding(
    icp: &InstallIcpContext,
    manifest: &PersistedCanicInfrastructureArtifactManifest,
    role: CanicInfrastructureRole,
) -> Result<ResolvedProtocolBinding, ProtocolBindingError> {
    let artifact = manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .ok_or_else(|| ProtocolBindingError::MissingBinding {
            canister: role.as_str().to_string(),
        })?;
    resolve_infrastructure_protocol_binding(icp.root(), icp.environment(), artifact)
}
