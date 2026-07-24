//! Module: dto::fleet_activation
//!
//! Responsibility: carry Fleet activation identity and evidence across host/runtime boundaries.
//! Does not own: validation, persistence, hashing, activation policy, or recovery transitions.
//! Boundary: authoritative owners validate these passive shapes before storing or acting on them.

use crate::ids::{FleetBinding, FleetKey, ReleaseBuildId};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// CurrentRootInstallIdentity
///

/// Exact current-only identity accepted by a fresh or reinstalled Fleet root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CurrentRootInstallIdentity {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub expected_module_hash: Option<[u8; 32]>,
}

///
/// FleetCascadeManifestEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetCascadeManifestEntry {
    pub principal: Principal,
    pub state_snapshot_hash: [u8; 32],
    pub topology_snapshot_hash: [u8; 32],
}

///
/// FleetCredentialGenerationRef
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FleetCredentialGenerationRef {
    pub generation: u64,
    pub manifest_hash: [u8; 32],
}

///
/// FleetCredentialManifestEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetCredentialManifestEntry {
    pub root_issuer: Principal,
    pub subject_canister: Principal,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub key_identity_hash: [u8; 32],
    pub cert_hash: [u8; 32],
    pub proof_hash: [u8; 32],
    pub bundle_hash: [u8; 32],
}

///
/// FleetCredentialManifest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetCredentialManifest {
    pub fleet: FleetKey,
    pub activation_id: [u8; 32],
    pub generation: u64,
    pub root_policy_set_hash: [u8; 32],
    pub renewal_template_set_hash: [u8; 32],
    pub entries: Vec<FleetCredentialManifestEntry>,
}

///
/// FleetActivationIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationIdentity {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// FleetHostCanisterActivationEvidence
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetHostCanisterActivationEvidence {
    pub principal: Principal,
    pub activation_evidence_hash: Option<[u8; 32]>,
}

///
/// FleetActivationHostRecord
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationHostRecord {
    pub identity: FleetActivationIdentity,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntry>>,
    pub credential: Option<FleetCredentialGenerationRef>,
    pub credential_manifest: Option<FleetCredentialManifest>,
    pub canisters: Vec<FleetHostCanisterActivationEvidence>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AppId, CanonicalNetworkId, FleetId, FleetKey, ReleaseBuildNonce};

    #[test]
    fn current_root_install_identity_roundtrips_with_text_ids() {
        let input = CurrentRootInstallIdentity {
            fleet: FleetBinding {
                fleet: FleetKey {
                    network: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([7; 32]),
                },
                app: AppId::from("toko"),
            },
            install_id: [8; 32],
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [9; 32],
            )),
            expected_module_hash: Some([10; 32]),
        };

        let bytes = candid::encode_one(&input).expect("encode current root install identity");
        let decoded: CurrentRootInstallIdentity =
            candid::decode_one(&bytes).expect("decode current root install identity");

        assert_eq!(decoded, input);
    }
}
