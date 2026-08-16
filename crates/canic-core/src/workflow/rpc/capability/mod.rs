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

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::{
        capability::{
            CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
        },
        error::Error,
        rpc::RootRequestMetadata,
    },
    ops::runtime::metrics::root_capability::RootCapabilityMetricProofMode,
    workflow::rpc::{
        RootCapabilityAuthority, RootCapabilityLifecycleExecutor,
        request::handler::capability::RootCapability,
    },
};

const MAX_CAPABILITY_CLOCK_SKEW_NS: u64 = 30_000_000_000;

const fn metric_proof_mode(proof: &CapabilityProof) -> RootCapabilityMetricProofMode {
    match proof {
        CapabilityProof::Structural => RootCapabilityMetricProofMode::Structural,
    }
}

/// Handle a v1 non-root cycles capability envelope.
pub async fn response_capability_v1_nonroot(
    envelope: NonrootCyclesCapabilityEnvelopeV1,
) -> Result<NonrootCyclesCapabilityResponseV1, InternalError> {
    nonroot::response_capability_v1_nonroot(envelope)
        .await
        .map_err(InternalError::observed_public)
}

/// Handle a v1 root capability envelope.
pub async fn response_capability_v1_root(
    envelope: crate::dto::capability::RootCapabilityEnvelopeV1,
    authority: RootCapabilityAuthority,
    lifecycle: &dyn RootCapabilityLifecycleExecutor,
) -> Result<crate::dto::capability::RootCapabilityResponseV1, InternalError> {
    root::response_capability_v1_root(envelope, authority, lifecycle)
        .await
        .map_err(InternalError::observed_public)
}

fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    envelope::validate_root_capability_envelope(service, capability_version, proof)
}

fn validate_nonroot_cycles_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    envelope::validate_root_capability_envelope(service, capability_version, proof)
}

fn verify_root_capability_proof(
    capability: &RootCapability,
    authority: &RootCapabilityAuthority,
) -> Result<(), Error> {
    proof::verify_root_structural_proof(capability, authority)
}

fn verify_nonroot_cycles_proof() -> Result<(), Error> {
    proof::verify_nonroot_structural_cycles_proof()
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
