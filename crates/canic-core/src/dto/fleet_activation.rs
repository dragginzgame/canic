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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCascadeManifestEntry {
    pub principal: Principal,
    pub state_snapshot_hash: [u8; 32],
    pub topology_snapshot_hash: [u8; 32],
}

///
/// FleetCascadeActivationEvidence
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetCascadeActivationEvidence {
    Source {
        cascade_manifest_hash: [u8; 32],
    },
    Applied {
        state_snapshot_hash: [u8; 32],
        topology_snapshot_hash: [u8; 32],
    },
}

///
/// FleetCredentialGenerationRef
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCredentialGenerationRef {
    pub generation: u64,
    pub manifest_hash: [u8; 32],
}

///
/// FleetCredentialManifestEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetActivationIdentity {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// FleetActivationPhase
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FleetActivationPhase {
    Prepared,
    Active,
}

///
/// FleetActivationStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetActivationStatusResponse {
    pub phase: FleetActivationPhase,
    pub identity: FleetActivationIdentity,
    pub cascade: Option<FleetCascadeActivationEvidence>,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntry>>,
    pub credential: Option<FleetCredentialGenerationRef>,
    pub credential_manifest: Option<FleetCredentialManifest>,
    pub activated_at_ns: Option<u64>,
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

    fn fleet_binding() -> FleetBinding {
        FleetBinding {
            fleet: FleetKey {
                network: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([7; 32]),
            },
            app: AppId::from("toko"),
        }
    }

    #[test]
    fn current_root_install_identity_roundtrips_with_text_ids() {
        let input = CurrentRootInstallIdentity {
            fleet: fleet_binding(),
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

    #[test]
    fn fleet_activation_status_roundtrips_as_named_candid_shapes() {
        let input = FleetActivationStatusResponse {
            phase: FleetActivationPhase::Prepared,
            identity: FleetActivationIdentity {
                fleet: fleet_binding(),
                operation_id: [11; 32],
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [12; 32],
                )),
            },
            cascade: Some(FleetCascadeActivationEvidence::Source {
                cascade_manifest_hash: [13; 32],
            }),
            cascade_manifest: Some(vec![FleetCascadeManifestEntry {
                principal: Principal::from_slice(&[14; 29]),
                state_snapshot_hash: [15; 32],
                topology_snapshot_hash: [16; 32],
            }]),
            credential: None,
            credential_manifest: None,
            activated_at_ns: None,
        };

        let bytes = candid::encode_one(&input).expect("encode activation status");
        let decoded: FleetActivationStatusResponse =
            candid::decode_one(&bytes).expect("decode activation status");

        assert_eq!(decoded, input);
    }
}
