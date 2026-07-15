//! Module: workflow::rpc::capability::verifier
//!
//! Responsibility: route typed capability proofs to verifier implementations.
//! Does not own: envelope validation, request dispatch, or capability DTO schemas.
//! Boundary: applies proof-mode-specific checks against immutable request context.

use crate::{
    dto::error::Error,
    workflow::rpc::{
        capability::{RootCapabilityProof, proof::verify_root_structural_proof},
        request::handler::capability::RootCapability,
    },
};
use async_trait::async_trait;

///
/// VerifiedCapability
///
/// Marker output for successful proof verification.
///

pub(super) struct VerifiedCapability;

///
/// VerificationInput
///
/// Immutable verification context shared by all proof verifiers.
///

pub(super) struct VerificationInput<'a> {
    pub(super) capability: &'a RootCapability,
}

///
/// CapabilityProofVerifier
///
/// Pluggable proof verifier contract for capability envelopes.
///

#[async_trait]
pub(super) trait CapabilityProofVerifier {
    /// Verify one proof mode against shared capability context.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error>;
}

///
/// StructuralVerifier
///
/// Verifies topology-only structural proofs.
///

struct StructuralVerifier;

#[async_trait]
impl CapabilityProofVerifier for StructuralVerifier {
    /// Validate structural preconditions for supported capability families.
    async fn verify(&self, input: &VerificationInput<'_>) -> Result<VerifiedCapability, Error> {
        verify_root_structural_proof(input.capability)?;
        Ok(VerifiedCapability)
    }
}

///
/// verify_root_capability_proof
///
/// Route proof verification through the mode-specific verifier implementation.
///
pub(super) async fn verify_root_capability_proof(
    capability: &RootCapability,
    _capability_version: u16,
    proof: RootCapabilityProof,
) -> Result<VerifiedCapability, Error> {
    let input = VerificationInput { capability };

    match proof {
        RootCapabilityProof::Structural => StructuralVerifier.verify(&input).await,
    }
}
