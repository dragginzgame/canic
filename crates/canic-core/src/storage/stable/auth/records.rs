use crate::ids::FleetKey;
use crate::storage::prelude::*;

///
/// LocalApplicationSessionRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalApplicationSessionRecord {
    pub transport_caller: Principal,
    pub authenticated_subject: Principal,
    pub issuer: Principal,
    pub fleet: FleetKey,
    pub role: CanisterRole,
    pub scopes: Vec<String>,
    pub authority_generation: u64,
    pub established_at_ns: u64,
    pub expires_at_ns: u64,
    pub proof_fingerprint: [u8; 32],
    pub establishment_request_hash: [u8; 32],
}

///
/// LocalApplicationReplayRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalApplicationReplayRecord {
    pub proof_fingerprint: [u8; 32],
    pub transport_caller: Principal,
    pub authenticated_subject: Principal,
    pub authority_generation: u64,
    pub remove_at_ns: u64,
}

/// Last locally activated protected binding used only for generation transitions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LocalApplicationAuthorityBindingRecord {
    Disabled,
    Enabled {
        fleet: FleetKey,
        role: CanisterRole,
        verifier_root_canister_id: Principal,
        minimum_accepted_registry_epoch: Option<u64>,
        allowed_scopes: Vec<String>,
        maximum_session_ttl_secs: u64,
    },
}

/// Atomic persisted projection for local application session authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalApplicationAuthorizationStateData {
    pub sessions: Vec<LocalApplicationSessionRecord>,
    pub replays: Vec<LocalApplicationReplayRecord>,
    pub authority_generation: u64,
    pub authority_binding: Option<LocalApplicationAuthorityBindingRecord>,
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
    pub aud: FleetKey,
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
    pub audience: FleetKey,
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
    pub allowed_audiences: Vec<FleetKey>,
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
    pub aud: FleetKey,
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
    pub application_sessions: Vec<LocalApplicationSessionRecord>,

    pub application_replays: Vec<LocalApplicationReplayRecord>,

    pub application_authority_generation: u64,

    pub application_authority_binding: Option<LocalApplicationAuthorityBindingRecord>,

    pub active_delegation_proof: Option<ActiveDelegationProofRecord>,

    pub root_issuers: Vec<RootIssuerRecord>,

    pub delegated_auth_registry_epoch: u64,

    pub delegated_auth_proof_epoch: u64,

    pub root_issuer_renewal_templates: Vec<RootIssuerRenewalTemplateRecord>,

    pub root_issuer_renewal_states: Vec<RootIssuerRenewalStateRecord>,

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
mod current_resource_contract {
    use super::*;

    fn encoded<T: Serialize>(value: &T) -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(value, &mut bytes).expect("CBOR encoding");
        bytes
    }

    fn principal(id: u64) -> Principal {
        let mut bytes = [0_u8; 29];
        bytes[..8].copy_from_slice(&id.to_be_bytes());
        Principal::from_slice(&bytes)
    }

    fn session(id: u64) -> LocalApplicationSessionRecord {
        let mut proof_fingerprint = [0_u8; 32];
        proof_fingerprint[..8].copy_from_slice(&id.to_be_bytes());
        LocalApplicationSessionRecord {
            transport_caller: principal(id),
            authenticated_subject: principal(id),
            issuer: principal(9_000),
            fleet: crate::test::support::fleet_key(1),
            role: CanisterRole::new("component"),
            scopes: vec!["app:read".to_string()],
            authority_generation: 1,
            established_at_ns: 1,
            expires_at_ns: 1_800_000_000_001,
            proof_fingerprint,
            establishment_request_hash: [2; 32],
        }
    }

    fn replay(id: u64) -> LocalApplicationReplayRecord {
        let mut proof_fingerprint = [0_u8; 32];
        proof_fingerprint[..8].copy_from_slice(&id.to_be_bytes());
        LocalApplicationReplayRecord {
            proof_fingerprint,
            transport_caller: principal(id),
            authenticated_subject: principal(id),
            authority_generation: 1,
            remove_at_ns: 60_000_000_001,
        }
    }

    fn maximum_scope_session(id: u64) -> LocalApplicationSessionRecord {
        let mut session = session(id);
        session.scopes = (0..16)
            .map(|scope| format!("app{scope:02}:{}", "x".repeat(58)))
            .collect();
        session
    }

    fn maximum_authority_binding() -> LocalApplicationAuthorityBindingRecord {
        LocalApplicationAuthorityBindingRecord::Enabled {
            fleet: crate::test::support::fleet_key(1),
            role: CanisterRole::new("component"),
            verifier_root_canister_id: principal(9_001),
            minimum_accepted_registry_epoch: Some(u64::MAX),
            allowed_scopes: maximum_scope_session(1).scopes,
            maximum_session_ttl_secs: 1_800,
        }
    }

    #[test]
    fn current_session_and_replay_cbor_footprint_stays_bounded() {
        let empty = AuthStateRecord::default();
        let one_session = AuthStateRecord {
            application_sessions: vec![session(1)],
            ..AuthStateRecord::default()
        };
        let one_replay = AuthStateRecord {
            application_replays: vec![replay(1)],
            ..AuthStateRecord::default()
        };
        let maximum = AuthStateRecord {
            application_sessions: (0..2_048).map(maximum_scope_session).collect(),
            application_replays: (0..4_096).map(replay).collect(),
            application_authority_generation: 1,
            application_authority_binding: Some(maximum_authority_binding()),
            ..AuthStateRecord::default()
        };

        let empty_bytes = encoded(&empty).len();
        let session_bytes = encoded(&session(1)).len();
        let replay_bytes = encoded(&replay(1)).len();
        assert_eq!(empty_bytes, 308);
        assert_eq!(session_bytes, 519);
        assert_eq!(replay_bytes, 198);
        assert_eq!(encoded(&maximum).len(), 4_025_450);
        assert!(session_bytes <= 2_048);
        assert!(encoded(&one_session).len() > empty_bytes);
        assert!(encoded(&one_replay).len() > empty_bytes);
        assert!(replay_bytes < session_bytes);
        assert!(encoded(&maximum).len() <= 8 * 1024 * 1024);
    }
}
