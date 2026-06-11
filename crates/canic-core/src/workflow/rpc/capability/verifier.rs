use crate::{dto::error::Error, dto::rpc::Request};
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

/// verify_root_capability_proof
///
/// Route proof verification through the mode-specific verifier implementation.
pub(super) async fn verify_root_capability_proof(
    capability: &Request,
    _capability_version: u16,
    proof: RootCapabilityProof,
) -> Result<VerifiedCapability, Error> {
    let input = VerificationInput { capability };

    match proof {
        RootCapabilityProof::Structural => StructuralVerifier.verify(&input).await,
    }
}
