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
/// IcCanisterSignatureProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcCanisterSignatureProofRecord {
    pub signature_cbor: Vec<u8>,
    pub public_key_der: Vec<u8>,
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

#[expect(
    clippy::large_enum_variant,
    reason = "RootProofRecord mirrors the stable delegated-auth proof schema; boxing would change the persisted record shape"
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProofRecord {
    IcCanisterSignatureV1(IcCanisterSignatureProofRecord),
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

///
/// RootIssuerRenewalOutcomeRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootIssuerRenewalOutcomeRecord {
    AlreadyInstalled,
    DriftDetected,
    InstallDeadlineExpired,
    Installed,
    IssuerCallFailed,
    NeverRun,
    PolicyRejected,
    ProofMismatch,
    QuotaExceeded,
    RejectedByIssuer,
    RetrievalExpired,
    TemplateChanged,
    TemplateDisabled,
}

///
/// RootIssuerRenewalStateRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStateRecord {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub active_attempt_id: Option<[u8; 32]>,
    pub last_outcome: RootIssuerRenewalOutcomeRecord,
    pub consecutive_failures: u32,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

///
/// RootIssuerRenewalProofRefRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalProofRefRecord {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
}

///
/// RootIssuerRenewalAttemptStatusRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootIssuerRenewalAttemptStatusRecord {
    Prepared,
    Installing,
    Installed,
    FailedRetryable,
    FailedTerminal,
    Disabled,
    Expired,
}

///
/// RootIssuerRenewalAttemptRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalAttemptRecord {
    pub attempt_id: [u8; 32],
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub batch_id: [u8; 32],
    pub proof_ref: RootIssuerRenewalProofRefRecord,
    pub status: RootIssuerRenewalAttemptStatusRecord,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
    pub install_deadline_ns: u64,
    pub prepared_cert_hash: [u8; 32],
    pub prepared_expires_at_ns: u64,
    pub prepared_refresh_after_ns: u64,
    pub failure: Option<RootIssuerRenewalOutcomeRecord>,
}

///
/// RootDelegationRenewalBatchRecord
///
/// Historical bridge-backed renewal batch state retained only for stable-state
/// decode. Active 0.76 renewal state uses `ChainKeyRootDelegationBatchRecord`.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalBatchRecord {
    pub batch_id: [u8; 32],
    pub attempt_ids: Vec<[u8; 32]>,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
}

///
/// RootProvisionerRecord
///
/// Historical renewal-provisioner ACL state retained only for stable-state
/// decode. Active 0.76 delegated auth has no provisioner ACL.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootProvisionerRecord {
    pub principal: Principal,
    pub enabled: bool,
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
    pub root_issuer_renewal_attempts: Vec<RootIssuerRenewalAttemptRecord>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_delegation_renewal_batches: Vec<RootDelegationRenewalBatchRecord>,

    #[serde(default)]
    pub chain_key_root_delegation_batches: Vec<ChainKeyRootDelegationBatchRecord>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_provisioners: Vec<RootProvisionerRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_cbor::{Value, value::to_value};
    use std::collections::BTreeSet;

    #[derive(Serialize)]
    struct AuthStateRecordWithoutHistoricalBridgeFields {
        delegated_sessions: Vec<DelegatedSessionRecord>,
        delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,
        active_delegation_proof: Option<ActiveDelegationProofRecord>,
        root_issuers: Vec<RootIssuerRecord>,
        delegated_auth_registry_epoch: u64,
        delegated_auth_proof_epoch: u64,
        root_issuer_renewal_templates: Vec<RootIssuerRenewalTemplateRecord>,
        root_issuer_renewal_states: Vec<RootIssuerRenewalStateRecord>,
        root_issuer_renewal_attempts: Vec<RootIssuerRenewalAttemptRecord>,
        chain_key_root_delegation_batches: Vec<ChainKeyRootDelegationBatchRecord>,
    }

    fn auth_state_keys(record: &AuthStateRecord) -> BTreeSet<String> {
        let value = to_value(record).expect("auth state should serialize to CBOR value");
        let Value::Map(map) = value else {
            panic!("auth state should serialize as a CBOR map");
        };

        map.into_keys()
            .map(|key| {
                let Value::Text(text) = key else {
                    panic!("auth state keys should serialize as text");
                };
                text
            })
            .collect()
    }

    #[test]
    fn empty_historical_bridge_auth_state_is_not_serialized() {
        let keys = auth_state_keys(&AuthStateRecord::default());

        assert!(!keys.contains("root_delegation_renewal_batches"));
        assert!(!keys.contains("root_provisioners"));
    }

    #[test]
    fn missing_historical_bridge_auth_state_decodes_to_empty_defaults() {
        let encoded = serde_cbor::to_vec(&AuthStateRecordWithoutHistoricalBridgeFields {
            delegated_sessions: Vec::new(),
            delegated_session_bootstrap_bindings: Vec::new(),
            active_delegation_proof: None,
            root_issuers: Vec::new(),
            delegated_auth_registry_epoch: 7,
            delegated_auth_proof_epoch: 8,
            root_issuer_renewal_templates: Vec::new(),
            root_issuer_renewal_states: Vec::new(),
            root_issuer_renewal_attempts: Vec::new(),
            chain_key_root_delegation_batches: Vec::new(),
        })
        .expect("auth state without historical bridge fields should encode");

        let decoded: AuthStateRecord = serde_cbor::from_slice(&encoded)
            .expect("auth state should decode without bridge fields");

        assert_eq!(decoded.delegated_auth_registry_epoch, 7);
        assert_eq!(decoded.delegated_auth_proof_epoch, 8);
        assert!(decoded.root_delegation_renewal_batches.is_empty());
        assert!(decoded.root_provisioners.is_empty());
    }

    #[test]
    fn nonempty_historical_bridge_auth_state_still_round_trips() {
        let record = AuthStateRecord {
            root_delegation_renewal_batches: vec![RootDelegationRenewalBatchRecord {
                batch_id: [1; 32],
                attempt_ids: vec![[2; 32]],
                prepared_at_ns: 3,
                retrieval_expires_at_ns: 4,
            }],
            root_provisioners: vec![RootProvisionerRecord {
                principal: Principal::from_slice(&[5; 29]),
                enabled: true,
            }],
            ..AuthStateRecord::default()
        };
        let keys = auth_state_keys(&record);

        assert!(keys.contains("root_delegation_renewal_batches"));
        assert!(keys.contains("root_provisioners"));

        let encoded = serde_cbor::to_vec(&record).expect("auth state should encode");
        let decoded: AuthStateRecord =
            serde_cbor::from_slice(&encoded).expect("auth state should decode");

        assert_eq!(decoded.root_delegation_renewal_batches.len(), 1);
        assert_eq!(decoded.root_delegation_renewal_batches[0].batch_id, [1; 32]);
        assert_eq!(decoded.root_provisioners.len(), 1);
        assert_eq!(
            decoded.root_provisioners[0].principal,
            Principal::from_slice(&[5; 29])
        );
    }
}
