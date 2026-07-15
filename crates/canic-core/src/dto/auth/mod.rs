//! Module: dto::auth
//!
//! Responsibility: expose passive auth boundary contracts.
//! Does not own: auth verification, persistence, policy, or runtime effects.
//! Boundary: stable Candid/Serde auth shapes re-exported from concern modules.

mod attestation;
mod common;
mod proof;
mod renewal;
#[cfg(test)]
mod tests;
mod token;

pub use attestation::{
    RoleAttestation, RoleAttestationGetRequest, RoleAttestationPrepareResponse,
    RoleAttestationRequest, RoleAttestationRootProof, SignedRoleAttestation,
};
pub use common::{AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience};
pub use proof::{
    ActiveDelegationProof, ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
    ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1,
    ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
    DelegatedAuthIssuerPolicySnapshotV1, DelegatedAuthRegistrySnapshotV1, DelegationCert,
    DelegationProof, IcCanisterSignatureProofV1, IcChainKeyBatchSignatureProofV1,
    InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse, IssuerProof,
    IssuerProofAlgorithm, IssuerProofBinding, RootKeyPolicyV1, RootProof, RootProofMode,
};
pub use renewal::{
    RootDelegationProofBatchProof, RootDelegationProofBatchProofRef, RootIssuerPolicyResponse,
    RootIssuerPolicyUpsertRequest, RootIssuerPolicyView, RootIssuerRenewalAttemptStatus,
    RootIssuerRenewalAttemptView, RootIssuerRenewalOutcome, RootIssuerRenewalStateView,
    RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
    RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
    RootIssuerRenewalTemplateView,
};
pub use token::{
    DelegatedToken, DelegatedTokenClaims, DelegatedTokenGetRequest, DelegatedTokenPrepareRequest,
    DelegatedTokenPrepareResponse,
};
