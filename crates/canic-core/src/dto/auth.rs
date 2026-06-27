use crate::dto::{prelude::*, rpc::RootRequestMetadata};

//
// DelegationAudience
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationAudience {
    Canister(Principal),
    CanicSubnet(Principal),
    Project(String),
}

//
// DelegatedRoleGrant
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

//
// RootProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}

//
// IssuerProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}

//
// IcCanisterSignatureProofV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcCanisterSignatureProofV1 {
    pub signature_cbor: Vec<u8>,
    pub public_key_der: Vec<u8>,
}

//
// IssuerProofAlgorithm
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofAlgorithm {
    IcCanisterSignatureV1,
}

//
// IssuerProofBinding
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofBinding {
    IcCanisterSignatureV1 { seed_hash: [u8; 32] },
}

//
// DelegationCert
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationCert {
    pub root_pid: Principal,
    pub issuer_pid: Principal,
    pub issuer_proof_alg: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBinding,
    pub issued_at_ns: u64,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub max_token_ttl_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
}

//
// DelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_proof: RootProof,
}

//
// ActiveDelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDelegationProof {
    pub proof: DelegationProof,
    pub cert_hash: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
    pub installed_at_ns: u64,
    pub installed_by: Principal,
}

//
// InstallActiveDelegationProofRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallActiveDelegationProofRequest {
    pub proof: DelegationProof,
}

//
// InstallActiveDelegationProofResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallActiveDelegationProofResponse {
    pub active_proof: ActiveDelegationProof,
}

//
// ActiveDelegationProofStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveDelegationProofStatus {
    Missing,
    Valid,
    RefreshNeeded,
    Expired,
}

//
// ActiveDelegationProofStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDelegationProofStatusResponse {
    pub status: ActiveDelegationProofStatus,
    pub root_pid: Option<Principal>,
    pub issuer_pid: Option<Principal>,
    pub cert_hash: Option<[u8; 32]>,
    pub expires_at_ns: Option<u64>,
    pub refresh_after_ns: Option<u64>,
}

//
// DelegatedTokenClaims
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenClaims {
    pub subject: Principal,
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub nonce: [u8; 16],
    #[serde(default)]
    pub ext: Option<Vec<u8>>,
}

//
// DelegatedToken
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub proof: DelegationProof,
    pub issuer_proof: IssuerProof,
}

//
// AuthRequestMetadata
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_ns: u64,
}

//
// RootDelegationProofBatchPrepareRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchPrepareRequest {
    #[serde(default)]
    pub metadata: Option<AuthRequestMetadata>,
    pub entries: Vec<RootDelegationProofBatchPrepareEntry>,
}

//
// RootDelegationProofBatchPrepareEntry
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchPrepareEntry {
    pub issuer_pid: Principal,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

//
// RootDelegationProofBatchPrepareResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchPrepareResponse {
    pub batch_id: [u8; 32],
    pub entries: Vec<RootDelegationProofBatchEntry>,
    pub retrieval_expires_at_ns: u64,
}

//
// RootDelegationProofBatchEntry
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchEntry {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
}

//
// RootDelegationProofBatchGetRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchGetRequest {
    pub batch_id: [u8; 32],
    pub entries: Vec<RootDelegationProofBatchProofRef>,
}

//
// RootDelegationRenewalProofBatchGetRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalProofBatchGetRequest {
    pub batch_id: [u8; 32],
}

//
// RootDelegationRenewalBatchView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalBatchView {
    pub batch_id: [u8; 32],
    pub attempt_count: u64,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
    pub install_deadline_ns: u64,
    pub attempts: Vec<RootIssuerRenewalAttemptView>,
}

//
// RootDelegationRenewalWorkListResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalWorkListResponse {
    pub batches: Vec<RootDelegationRenewalBatchView>,
}

//
// RootDelegationRenewalProvisionerUpsertRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalProvisionerUpsertRequest {
    pub principal: Principal,
    pub enabled: bool,
}

//
// RootDelegationRenewalProvisionerView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalProvisionerView {
    pub principal: Principal,
    pub enabled: bool,
}

//
// RootDelegationRenewalProvisionerResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalProvisionerResponse {
    pub provisioner: RootDelegationRenewalProvisionerView,
}

//
// RootDelegationRenewalProvisionerListResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationRenewalProvisionerListResponse {
    pub provisioners: Vec<RootDelegationRenewalProvisionerView>,
}

//
// RootDelegationProofBatchProofRef
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchProofRef {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
}

//
// RootDelegationProofBatchGetResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchGetResponse {
    pub batch_id: [u8; 32],
    pub proofs: Vec<RootDelegationProofBatchProof>,
}

//
// RootDelegationProofBatchProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchProof {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub proof: DelegationProof,
}

//
// RootDelegationProofBatchInstallRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchInstallRequest {
    pub batch_id: [u8; 32],
    pub proofs: Vec<RootDelegationProofBatchProof>,
}

//
// RootDelegationProofBatchInstallResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchInstallResponse {
    pub batch_id: [u8; 32],
    pub outcomes: Vec<RootDelegationProofBatchInstallResult>,
}

//
// RootDelegationProofBatchInstallResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootDelegationProofBatchInstallResult {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub outcome: RootDelegationProofInstallOutcome,
}

//
// RootDelegationProofInstallOutcome
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootDelegationProofInstallOutcome {
    Installed,
    AlreadyInstalled,
    RejectedBySigner,
    CallFailed,
    ProofMismatch,
    ExpiredOrSuperseded,
}

//
// RootIssuerPolicyUpsertRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyUpsertRequest {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

//
// RootIssuerPolicyView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

//
// RootIssuerPolicyResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerPolicyResponse {
    pub issuer: RootIssuerPolicyView,
}

//
// RootIssuerRenewalTemplateUpsertRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateUpsertRequest {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

//
// RootIssuerRenewalTemplateView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateView {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub cert_ttl_ns: u64,
}

//
// RootIssuerRenewalTemplateResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalTemplateResponse {
    pub template: RootIssuerRenewalTemplateView,
}

//
// RootIssuerRenewalStatusRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStatusRequest {
    pub issuer_pid: Principal,
}

//
// RootIssuerRenewalOutcome
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootIssuerRenewalOutcome {
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

//
// RootIssuerRenewalAttemptStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootIssuerRenewalAttemptStatus {
    Prepared,
    Installing,
    Installed,
    FailedRetryable,
    FailedTerminal,
    Disabled,
    Expired,
}

//
// RootIssuerRenewalAttemptView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalAttemptView {
    pub attempt_id: [u8; 32],
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub batch_id: [u8; 32],
    pub proof_ref: RootDelegationProofBatchProofRef,
    pub status: RootIssuerRenewalAttemptStatus,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
    pub install_deadline_ns: u64,
    pub prepared_cert_hash: [u8; 32],
    pub prepared_expires_at_ns: u64,
    pub prepared_refresh_after_ns: u64,
    pub failure: Option<RootIssuerRenewalOutcome>,
}

//
// RootIssuerRenewalStateView
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStateView {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub active_attempt_id: Option<[u8; 32]>,
    pub last_outcome: RootIssuerRenewalOutcome,
    pub consecutive_failures: u32,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

//
// RootIssuerRenewalStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootIssuerRenewalStatusResponse {
    pub template: Option<RootIssuerRenewalTemplateView>,
    pub state: Option<RootIssuerRenewalStateView>,
    pub active_attempt: Option<RootIssuerRenewalAttemptView>,
}

//
// DelegatedTokenPrepareRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenPrepareRequest {
    #[serde(default)]
    pub metadata: Option<AuthRequestMetadata>,
    pub subject: Principal,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    #[serde(default)]
    pub ext: Option<Vec<u8>>,
}

//
// DelegatedTokenPrepareResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenPrepareResponse {
    pub claims: DelegatedTokenClaims,
    pub claims_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// DelegatedTokenGetRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedTokenGetRequest {
    pub claims_hash: [u8; 32],
}

//
// RoleAttestationRequest
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RoleAttestationRequest {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub ttl_ns: u64,
    pub epoch: u64,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// RoleAttestation
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestation {
    pub subject: Principal,
    pub role: CanisterRole,
    #[serde(default)]
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub epoch: u64,
}

//
// RoleAttestationPrepareResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestationPrepareResponse {
    pub payload: RoleAttestation,
    pub payload_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

//
// RoleAttestationGetRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleAttestationGetRequest {
    pub payload_hash: [u8; 32],
}

//
// SignedRoleAttestation
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedRoleAttestation {
    pub payload: RoleAttestation,
    pub root_proof: RootProof,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn auth_dtos_remain_passive_boundary_types() {
        let source = include_str!("auth.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source exists");

        for marker in [
            "impl DelegatedToken",
            "impl DelegatedTokenClaims",
            "impl RoleAttestation",
            "impl SignedRoleAttestation",
            "fn verify",
            "fn sign",
            "fn resolve",
            "fn replay",
            "fn consume",
            "fn policy",
            "fn validate",
        ] {
            assert!(
                !production_source.contains(marker),
                "auth DTOs must stay passive; found marker `{marker}`"
            );
        }
    }
}
