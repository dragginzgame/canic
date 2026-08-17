//! Module: api::role
//!
//! Responsibility: adapt the compiled role contract into immutable status verification.
//! Does not own: capability derivation, endpoint emission, authorization, or mutable state.
//! Boundary: maps internal capability keys to the closed public role overview values.

use crate::{
    dto::{
        metadata::CanicMetadataResponse,
        role::{RoleCapability, RoleOverviewResponse},
        state::BootstrapStatusResponse,
    },
    ids::CanisterRole,
    role_contract::RoleCapabilityKey,
};
use std::collections::BTreeSet;

/// Endpoint-facing adapter for immutable compiled-role verification.
pub struct RoleOverviewApi;

impl RoleOverviewApi {
    /// Build one overview from the already resolved build-time capability authority.
    #[must_use]
    pub fn overview(
        role: CanisterRole,
        capabilities: &BTreeSet<RoleCapabilityKey>,
        protocol_profile_digest: [u8; 32],
        metadata: CanicMetadataResponse,
        bootstrap: BootstrapStatusResponse,
    ) -> RoleOverviewResponse {
        let mut capabilities = capabilities
            .iter()
            .copied()
            .map(capability_view)
            .collect::<Vec<_>>();
        capabilities.sort_unstable();

        RoleOverviewResponse {
            role,
            capabilities,
            protocol_profile_digest,
            metadata,
            bootstrap,
        }
    }
}

const fn capability_view(capability: RoleCapabilityKey) -> RoleCapability {
    match capability {
        RoleCapabilityKey::AutomaticTopup => RoleCapability::AutomaticTopup,
        RoleCapabilityKey::DelegatedTokenIssuer => RoleCapability::DelegatedTokenIssuer,
        RoleCapabilityKey::DelegatedTokenVerifier => RoleCapability::DelegatedTokenVerifier,
        RoleCapabilityKey::FleetCoordinator => RoleCapability::FleetCoordinator,
        RoleCapabilityKey::Index => RoleCapability::Index,
        RoleCapabilityKey::Icrc21 => RoleCapability::Icrc21,
        RoleCapabilityKey::RoleAttestationSigner => RoleCapability::RoleAttestationSigner,
        RoleCapabilityKey::RoleAttestationVerifier => RoleCapability::RoleAttestationVerifier,
        RoleCapabilityKey::Root => RoleCapability::Root,
        RoleCapabilityKey::RootControlPlane => RoleCapability::RootControlPlane,
        RoleCapabilityKey::Runtime => RoleCapability::Runtime,
        RoleCapabilityKey::Scaling => RoleCapability::Scaling,
        RoleCapabilityKey::Sharding => RoleCapability::Sharding,
        RoleCapabilityKey::WasmStore => RoleCapability::WasmStore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_projects_every_compiled_capability_in_lexical_order() {
        let capabilities = BTreeSet::from([
            RoleCapabilityKey::AutomaticTopup,
            RoleCapabilityKey::DelegatedTokenIssuer,
            RoleCapabilityKey::DelegatedTokenVerifier,
            RoleCapabilityKey::FleetCoordinator,
            RoleCapabilityKey::Index,
            RoleCapabilityKey::Icrc21,
            RoleCapabilityKey::RoleAttestationSigner,
            RoleCapabilityKey::RoleAttestationVerifier,
            RoleCapabilityKey::Root,
            RoleCapabilityKey::RootControlPlane,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::Scaling,
            RoleCapabilityKey::Sharding,
            RoleCapabilityKey::WasmStore,
        ]);
        let overview = RoleOverviewApi::overview(
            CanisterRole::new("app"),
            &capabilities,
            [9; 32],
            CanicMetadataResponse {
                package_name: "app".to_string(),
                package_version: "0.1.0".to_string(),
                package_description: "test".to_string(),
                canic_version: "0.103.0".to_string(),
                canister_version: 4,
            },
            BootstrapStatusResponse {
                ready: true,
                phase: "ready".to_string(),
                last_error: None,
            },
        );

        assert_eq!(
            overview.capabilities,
            vec![
                RoleCapability::AutomaticTopup,
                RoleCapability::DelegatedTokenIssuer,
                RoleCapability::DelegatedTokenVerifier,
                RoleCapability::FleetCoordinator,
                RoleCapability::Icrc21,
                RoleCapability::Index,
                RoleCapability::RoleAttestationSigner,
                RoleCapability::RoleAttestationVerifier,
                RoleCapability::Root,
                RoleCapability::RootControlPlane,
                RoleCapability::Runtime,
                RoleCapability::Scaling,
                RoleCapability::Sharding,
                RoleCapability::WasmStore,
            ]
        );
        assert_eq!(overview.protocol_profile_digest, [9; 32]);
    }
}
