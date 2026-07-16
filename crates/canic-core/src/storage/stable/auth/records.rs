use crate::storage::prelude::*;

///
/// DelegatedSessionRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bootstrap_token_fingerprint: Option<[u8; 32]>,
}

///
/// DelegatedSessionBootstrapBindingRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionBootstrapBindingRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub token_fingerprint: [u8; 32],
    pub bound_at: u64,
    pub expires_at: u64,
}

///
/// DelegationAudienceRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationAudienceRecord {
    Canister(Principal),
    CanicSubnet(Principal),
    Project(String),
}

///
/// DelegatedRoleGrantRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedRoleGrantRecord {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

///
/// IssuerProofAlgorithmRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofAlgorithmRecord {
    IcCanisterSignatureV1,
}

///
/// IssuerProofBindingRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofBindingRecord {
    IcCanisterSignatureV1 { seed_hash: [u8; 32] },
}

///
/// DelegationCertRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationCertRecord {
    pub root_pid: Principal,
    pub issuer_pid: Principal,
    pub issuer_proof_alg: IssuerProofAlgorithmRecord,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBindingRecord,
    pub issued_at_ns: u64,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub max_token_ttl_ns: u64,
    pub aud: DelegationAudienceRecord,
    pub grants: Vec<DelegatedRoleGrantRecord>,
}

///
/// ChainKeyAlgorithmRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChainKeyAlgorithmRecord {
    EcdsaSecp256k1,
}

///
/// ChainKeyKeyIdRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyKeyIdRecord {
    pub name: String,
}

///
/// ChainKeyBatchHeaderRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyBatchHeaderRecord {
    pub schema_version: u16,
    pub root_canister_id: Principal,
    pub batch_id: [u8; 32],
    pub proof_epoch: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
    pub tree_root: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub algorithm: ChainKeyAlgorithmRecord,
    pub key_id: ChainKeyKeyIdRecord,
    pub derivation_path_hash: [u8; 32],
    pub key_version: u64,
}

///
/// ChainKeyDelegationCertRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyDelegationCertRecord {
    pub root_canister_id: Principal,
    pub issuer_canister_id: Principal,
    pub proof_epoch: u64,
    pub issuer_proof_algorithm: IssuerProofAlgorithmRecord,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBindingRecord,
    pub max_token_ttl_ns: u64,
    pub audience: DelegationAudienceRecord,
    pub grants: Vec<DelegatedRoleGrantRecord>,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
}

///
/// ChainKeyRootSignatureRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyRootSignatureRecord {
    pub algorithm: ChainKeyAlgorithmRecord,
    pub key_id: ChainKeyKeyIdRecord,
    pub derivation_path: Vec<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

///
/// ChainKeyBatchWitnessRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyBatchWitnessRecord {
    pub steps: Vec<ChainKeyBatchWitnessStepRecord>,
}

///
/// ChainKeyBatchWitnessStepRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChainKeyBatchWitnessStepRecord {
    LeftSibling([u8; 32]),
    RightSibling([u8; 32]),
}

///
/// IcChainKeyBatchSignatureProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcChainKeyBatchSignatureProofRecord {
    pub header: ChainKeyBatchHeaderRecord,
    pub delegation_cert: ChainKeyDelegationCertRecord,
    pub issuer_witness: ChainKeyBatchWitnessRecord,
    pub signature: ChainKeyRootSignatureRecord,
}

///
/// ChainKeyRootDelegationBatchStatusRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChainKeyRootDelegationBatchStatusRecord {
    Prepared,
    Signing,
    Signed,
    Installing,
    Installed,
    FailedRetryable,
}

///
/// ChainKeyRootDelegationBatchIssuerRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyRootDelegationBatchIssuerRecord {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub delegation_cert: DelegationCertRecord,
    pub chain_key_delegation_cert: ChainKeyDelegationCertRecord,
    pub issuer_witness: ChainKeyBatchWitnessRecord,
    pub refresh_after_ns: u64,
    pub installed_at_ns: Option<u64>,
    pub last_failure: Option<String>,
}

///
/// ChainKeyRootDelegationBatchRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyRootDelegationBatchRecord {
    pub batch_id: [u8; 32],
    pub status: ChainKeyRootDelegationBatchStatusRecord,
    pub header_hash: [u8; 32],
    pub header: ChainKeyBatchHeaderRecord,
    pub signature: Option<ChainKeyRootSignatureRecord>,
    pub issuers: Vec<ChainKeyRootDelegationBatchIssuerRecord>,
    pub prepared_at_ns: u64,
    pub signed_at_ns: Option<u64>,
    pub install_started_at_ns: Option<u64>,
    pub installed_at_ns: Option<u64>,
    pub retry_after_ns: Option<u64>,
    pub failure: Option<String>,
}

///
/// RootProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProofRecord {
    IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofRecord),
}

///
/// DelegationProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProofRecord {
    pub cert: DelegationCertRecord,
    pub root_proof: RootProofRecord,
}

///
/// ActiveDelegationProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDelegationProofRecord {
    pub proof: DelegationProofRecord,
    pub cert_hash: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
    pub installed_at_ns: u64,
    pub installed_by: Principal,
}

///
/// RootIssuerRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRecord {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudienceRecord>,
    pub allowed_grants: Vec<DelegatedRoleGrantRecord>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

///
/// RootIssuerRenewalTemplateRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateRecord {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudienceRecord,
    pub grants: Vec<DelegatedRoleGrantRecord>,
    pub cert_ttl_ns: u64,
}

/// RootIssuerRenewalStateRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStateRecord {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

///
/// AuthStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AuthStateRecord {
    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    pub delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,

    #[serde(default)]
    pub active_delegation_proof: Option<ActiveDelegationProofRecord>,

    #[serde(default)]
    pub root_issuers: Vec<RootIssuerRecord>,

    #[serde(default)]
    pub delegated_auth_registry_epoch: u64,

    #[serde(default)]
    pub delegated_auth_proof_epoch: u64,

    #[serde(default)]
    pub root_issuer_renewal_templates: Vec<RootIssuerRenewalTemplateRecord>,

    #[serde(default)]
    pub root_issuer_renewal_states: Vec<RootIssuerRenewalStateRecord>,

    #[serde(default)]
    pub chain_key_root_delegation_batches: Vec<ChainKeyRootDelegationBatchRecord>,
}

impl AuthStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "AuthStateRecord";
}

///
/// AuthStateData
///
/// Canonical full auth-state snapshot used for schema and round-trip validation.
///

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "auth snapshots are materialized only by focused round-trip validation"
    )
)]
#[derive(Clone, Debug, Default)]
pub struct AuthStateData {
    pub record: AuthStateRecord,
}

impl AuthStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "AuthStateData";
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::Value;
    use std::collections::BTreeSet;

    fn auth_state_keys(record: &AuthStateRecord) -> BTreeSet<String> {
        let value = Value::serialized(record).expect("auth state should serialize to CBOR value");
        let Value::Map(map) = value else {
            panic!("auth state should serialize as a CBOR map");
        };

        map.into_iter()
            .map(|(key, _)| {
                let Value::Text(text) = key else {
                    panic!("auth state keys should serialize as text");
                };
                text
            })
            .collect()
    }

    #[test]
    fn historical_bridge_auth_state_fields_are_not_serialized() {
        let keys = auth_state_keys(&AuthStateRecord::default());

        assert!(!keys.contains("root_delegation_renewal_batches"));
        assert!(!keys.contains("root_provisioners"));
    }
}
