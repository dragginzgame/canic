//! Module: workflow::rpc::capability
//!
//! Responsibility: validate and dispatch capability-envelope RPC requests.
//! Does not own: endpoint authentication, request execution, or replay storage schema.
//! Boundary: maps capability DTOs into proof checks, replay metadata, and handler calls.

mod envelope;
mod nonroot;
mod proof;
mod replay;
mod root;
mod verifier;

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        capability::{
            CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
        },
        error::Error,
        rpc::{Request, RootRequestMetadata},
    },
    ops::{
        rpc::capability::root_capability_hash as compute_root_capability_hash,
        runtime::metrics::root_capability::RootCapabilityMetricProofMode,
    },
    workflow::rpc::request::handler::capability::RootCapability,
};

const REPLAY_REQUEST_ID_DOMAIN_V1: &[u8] = b"CANIC_REPLAY_REQUEST_ID_V1";
const MAX_CAPABILITY_CLOCK_SKEW_NS: u64 = 30_000_000_000;

///
/// RootCapabilityProofMode
///
/// Canonical classification for capability proof logging and metrics.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootCapabilityProofMode {
    Structural,
}

impl RootCapabilityProofMode {
    /// Classify a raw proof without validating its payload header.
    const fn from_proof(proof: &CapabilityProof) -> Self {
        match proof {
            CapabilityProof::Structural => Self::Structural,
        }
    }

    /// Human-readable label used in capability logs.
    const fn label(self) -> &'static str {
        match self {
            Self::Structural => "Structural",
        }
    }

    /// Metrics dimension matching the canonical proof classification.
    const fn metric_key(self) -> RootCapabilityMetricProofMode {
        match self {
            Self::Structural => RootCapabilityMetricProofMode::Structural,
        }
    }
}

///
/// RootCapabilityProof
///
/// Validated proof view used after envelope checks and before verification.
///

#[derive(Clone, Copy, Debug)]
pub(super) enum RootCapabilityProof {
    Structural,
}

impl RootCapabilityProof {
    /// Validate the proof wire header and expose the typed proof view.
    const fn validate(proof: &CapabilityProof) -> Self {
        match proof {
            CapabilityProof::Structural => Self::Structural,
        }
    }

    /// Return the canonical proof mode for this validated proof.
    const fn mode(self) -> RootCapabilityProofMode {
        match self {
            Self::Structural => RootCapabilityProofMode::Structural,
        }
    }
}

/// Handle a v1 non-root cycles capability envelope.
pub async fn response_capability_v1_nonroot(
    envelope: NonrootCyclesCapabilityEnvelopeV1,
) -> Result<NonrootCyclesCapabilityResponseV1, InternalError> {
    nonroot::response_capability_v1_nonroot(envelope)
        .await
        .map_err(InternalError::public)
}

/// Handle a v1 root capability envelope.
pub async fn response_capability_v1_root(
    envelope: crate::dto::capability::RootCapabilityEnvelopeV1,
) -> Result<crate::dto::capability::RootCapabilityResponseV1, InternalError> {
    root::response_capability_v1_root(envelope)
        .await
        .map_err(InternalError::public)
}

fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<RootCapabilityProof, Error> {
    envelope::validate_root_capability_envelope(service, capability_version, proof)
}

fn validate_nonroot_cycles_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<RootCapabilityProof, Error> {
    let proof = envelope::validate_root_capability_envelope(service, capability_version, proof)?;

    if proof.mode() != RootCapabilityProofMode::Structural {
        return Err(Error::forbidden(
            "non-root capability endpoint only supports structural proof mode",
        ));
    }

    Ok(proof)
}

async fn verify_root_capability_proof(
    capability: &RootCapability,
    capability_version: u16,
    proof: RootCapabilityProof,
) -> Result<(), Error> {
    verifier::verify_root_capability_proof(capability, capability_version, proof)
        .await
        .map(|_| ())
}

fn verify_nonroot_cycles_proof() -> Result<(), Error> {
    proof::verify_nonroot_structural_cycles_proof()
}

#[cfg(test)]
fn verify_capability_hash_binding(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
    capability_hash: [u8; 32],
) -> Result<(), Error> {
    proof::verify_capability_hash_binding(
        target_canister,
        capability_version,
        capability,
        capability_hash,
    )
}

/// Compute the canonical root capability hash for proof binding.
pub fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    compute_root_capability_hash(target_canister, capability_version, capability)
}

const fn with_root_request_metadata(
    request: RootCapability,
    metadata: RootRequestMetadata,
) -> RootCapability {
    replay::with_root_request_metadata(request, metadata)
}

fn project_replay_metadata(
    metadata: CapabilityRequestMetadata,
    now_ns: u64,
) -> Result<RootRequestMetadata, Error> {
    replay::project_replay_metadata(metadata, now_ns)
}
