use crate::{cdk::types::Principal, dto::error::Error, dto::rpc::Request};
use async_trait::async_trait;

use super::RootCapabilityProof;

/// VerifiedCapability
///
/// Marker output for successful proof verification.
pub(super) struct VerifiedCapability;

/// VerificationInput
///
/// Immutable verification context shared by all proof verifiers.
pub(super) struct VerificationInput<'a> {
    pub(super) capability: &'a Request,
    pub(super) capability_version: u16,
    pub(super) caller: Principal,
    pub(super) target_canister: Principal,
    pub(super) now_secs: u64,
}

/// CapabilityProofVerifier
///
/// Pluggable proof verifier contract for capability envelopes.
#[async_trait]
pub(super) trait CapabilityProofVerifier {
    /// Verify one proof mode against shared capability context.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error>;
}

/// StructuralVerifier
///
/// Verifies topology-only structural proofs.
struct StructuralVerifier;

#[async_trait]
impl CapabilityProofVerifier for StructuralVerifier {
    /// Validate structural preconditions for supported capability families.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error> {
        super::proof::verify_root_structural_proof(input.capability)?;
        Ok(VerifiedCapability)
    }
}

/// RoleAttestationVerifier
///
/// Verifies attestation proof with capability hash binding.
struct RoleAttestationVerifier<'a> {
    blob: &'a crate::dto::capability::CapabilityProofBlob,
}

#[async_trait]
impl CapabilityProofVerifier for RoleAttestationVerifier<'_> {
    /// Verify hash binding first, then run delegated attestation verification.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error> {
        let proof = super::proof::decode_role_attestation_blob(self.blob)?;

        super::proof::verify_capability_hash_binding(
            input.target_canister,
            input.capability_version,
            input.capability,
            proof.capability_hash,
        )?;

        crate::api::auth::AuthApi::verify_role_attestation(&proof.attestation, 0).await?;
        Ok(VerifiedCapability)
    }
}

/// DelegatedGrantVerifier
///
/// Verifies grant hash binding, claims, and signature for delegated grants.
struct DelegatedGrantVerifier<'a> {
    blob: &'a crate::dto::capability::CapabilityProofBlob,
}

#[async_trait]
impl CapabilityProofVerifier for DelegatedGrantVerifier<'_> {
    /// Keep existing delegated-grant verification ordering unchanged.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error> {
        let proof = super::proof::decode_delegated_grant_blob(self.blob)?;

        super::proof::verify_capability_hash_binding(
            input.target_canister,
            input.capability_version,
            input.capability,
            proof.capability_hash,
        )?;
        super::verify_delegated_grant_hash_binding(&proof)?;
        super::verify_root_delegated_grant_proof(
            input.capability,
            &proof,
            input.caller,
            input.target_canister,
            input.now_secs,
        )?;

        Ok(VerifiedCapability)
    }
}

/// verify_root_capability_proof
///
/// Route proof verification through the mode-specific verifier implementation.
pub(super) async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: RootCapabilityProof<'_>,
) -> Result<VerifiedCapability, Error> {
    let input = VerificationInput {
        capability,
        capability_version,
        caller: crate::ops::ic::IcOps::msg_caller(),
        target_canister: crate::ops::ic::IcOps::canister_self(),
        now_secs: crate::ops::ic::IcOps::now_secs(),
    };

    match proof {
        RootCapabilityProof::Structural => StructuralVerifier.verify(&input).await,
        RootCapabilityProof::RoleAttestation(blob) => {
            RoleAttestationVerifier { blob }.verify(&input).await
        }
        RootCapabilityProof::DelegatedGrant(blob) => {
            DelegatedGrantVerifier { blob }.verify(&input).await
        }
    }
}
