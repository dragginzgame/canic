//! Module: workflow::rpc::capability::root
//!
//! Responsibility: validate and dispatch root capability envelopes.
//! Does not own: request execution, replay storage schema, or endpoint authentication.
//! Boundary: coordinates envelope checks, proof verification, metrics, and root dispatch.

use crate::{
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::Error,
    },
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
    },
    workflow::rpc::{
        capability::{
            metric_proof_mode, project_replay_metadata, validate_root_capability_envelope,
            verify_root_capability_proof, with_root_request_metadata,
        },
        request::handler::{RootResponseWorkflow, capability::RootCapability},
    },
};

/// response_capability_v1_root
///
/// Execute the full root capability verifier and dispatcher path.
pub(super) async fn response_capability_v1_root(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let RootCapabilityEnvelopeV1 {
        service,
        capability_version,
        capability,
        proof,
        metadata,
    } = envelope;

    let capability = RootCapability::from_request(capability);
    let descriptor = capability.descriptor();
    let capability_key = descriptor.key;
    let proof_mode = metric_proof_mode(&proof);
    match validate_root_capability_envelope(service, capability_version, &proof) {
        Ok(()) => {}
        Err(err) => {
            RootCapabilityMetrics::record_envelope(
                capability_key,
                RootCapabilityMetricOutcome::Rejected,
                proof_mode,
            );
            log!(
                Topic::Rpc,
                Warn,
                "root capability envelope rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
                descriptor.name,
                IcOps::msg_caller(),
                service,
                capability_version,
                proof_mode.metric_label(),
                err
            );
            return Err(err);
        }
    }
    RootCapabilityMetrics::record_envelope(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode,
    );

    if let Err(err) = verify_root_capability_proof(&capability) {
        RootCapabilityMetrics::record_proof(
            capability_key,
            RootCapabilityMetricOutcome::Rejected,
            proof_mode,
        );
        log!(
            Topic::Rpc,
            Warn,
            "root capability proof rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            descriptor.name,
            IcOps::msg_caller(),
            service,
            capability_version,
            proof_mode.metric_label(),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record_proof(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode,
    );

    let replay_metadata = project_replay_metadata(metadata, IcOps::now_nanos())?;
    let capability = with_root_request_metadata(capability, replay_metadata);
    let response = RootResponseWorkflow::response_capability_replay_first(capability)
        .await
        .map_err(Error::from)?;

    Ok(RootCapabilityResponseV1 { response })
}
