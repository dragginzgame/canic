//! Module: dto::auth
//!
//! Responsibility: expose passive auth boundary contracts.
//! Does not own: auth verification, persistence, policy, or runtime effects.
//! Boundary: stable Candid/Serde auth shapes re-exported from concern modules.

mod attestation;
mod common;
mod enrollment;
mod proof;
mod renewal;

pub use attestation::{
    RoleAttestation, RoleAttestationGetRequest, RoleAttestationPrepareResponse,
    RoleAttestationRequest, RoleAttestationRootProof, SignedRoleAttestation,
};
pub use common::{AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience};
pub use enrollment::{
    ApplicationEnrollmentBinding, ApplicationEnrollmentEntry, ApplicationEnrollmentKey,
    ApplicationEnrollmentRequest, ApplicationEnrollmentSnapshot,
};
pub use proof::{
    ActiveDelegationProof, ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
    ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1,
    ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
    DelegatedAuthIssuerPolicySnapshotV1, DelegatedAuthRegistrySnapshotV1, DelegationCert,
    DelegationProof, IcCanisterSignatureProofV1, IcChainKeyBatchSignatureProofV1,
    InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
    IssuerProofAlgorithm, IssuerProofBinding, RootChainKeyPublicKeyRequest, RootKeyPolicyV1,
    RootProof,
};
pub use renewal::{
    RootDelegationProofBatchProof, RootIssuerConfigureRequest, RootIssuerConfigureResponse,
    RootIssuerPolicyView, RootIssuerRenewalBatchStatus, RootIssuerRenewalBatchView,
    RootIssuerRenewalStateView, RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
    RootIssuerRenewalTemplateView,
};

#[cfg(test)]
mod tests;
