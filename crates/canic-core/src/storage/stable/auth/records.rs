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
/// RootProofRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProofRecord {
    IcCanisterSignatureV1(IcCanisterSignatureProofRecord),
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
    pub root_issuer_renewal_templates: Vec<RootIssuerRenewalTemplateRecord>,

    #[serde(default)]
    pub root_issuer_renewal_states: Vec<RootIssuerRenewalStateRecord>,

    #[serde(default)]
    pub root_issuer_renewal_attempts: Vec<RootIssuerRenewalAttemptRecord>,

    #[serde(default)]
    pub root_delegation_renewal_batches: Vec<RootDelegationRenewalBatchRecord>,

    #[serde(default)]
    pub root_provisioners: Vec<RootProvisionerRecord>,
}
