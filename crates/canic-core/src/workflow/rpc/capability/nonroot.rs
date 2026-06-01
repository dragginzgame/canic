use super::{
    RootCapabilityProofMode, project_replay_metadata, validate_nonroot_cycles_envelope,
    verify_nonroot_cycles_proof,
};
use crate::{
    dto::{
        capability::{NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1},
        error::Error,
    },
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
        },
    },
    workflow::rpc::request::handler::NonrootCyclesCapabilityWorkflow,
};

/// Validate and execute the non-root request-cycles capability path.
pub(super) async fn response_capability_v1_nonroot(
    envelope: NonrootCyclesCapabilityEnvelopeV1,
) -> Result<NonrootCyclesCapabilityResponseV1, Error> {
    let NonrootCyclesCapabilityEnvelopeV1 {
        service,
        capability_version,
        capability,
        proof,
        metadata,
    } = envelope;

    let capability_key = RootCapabilityMetricKey::RequestCycles;
    let proof_mode = RootCapabilityProofMode::from_proof(&proof);
    let validated_proof = match validate_nonroot_cycles_envelope(
        service,
        capability_version,
        &proof,
    ) {
        Ok(proof) => proof,
        Err(err) => {
            RootCapabilityMetrics::record_envelope(
                capability_key,
                RootCapabilityMetricOutcome::Rejected,
                proof_mode.metric_key(),
            );
            log!(
                Topic::Rpc,
                Warn,
                "non-root capability envelope rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
                "RequestCycles",
                IcOps::msg_caller(),
                service,
                capability_version,
                proof_mode.label(),
                err
            );
            return Err(err);
        }
    };
    let proof_mode = validated_proof.mode();
    RootCapabilityMetrics::record_envelope(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode.metric_key(),
    );

    if let Err(err) = verify_nonroot_cycles_proof() {
        RootCapabilityMetrics::record_proof(
            capability_key,
            RootCapabilityMetricOutcome::Rejected,
            proof_mode.metric_key(),
        );
        log!(
            Topic::Rpc,
            Warn,
            "non-root capability proof rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            "RequestCycles",
            IcOps::msg_caller(),
            service,
            capability_version,
            proof_mode.label(),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record_proof(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode.metric_key(),
    );

    let replay_metadata = project_replay_metadata(metadata, IcOps::now_secs())?;
    let mut request = capability;
    request.metadata = Some(replay_metadata);
    let response = NonrootCyclesCapabilityWorkflow::response_replay_first(request)
        .await
        .map_err(Error::from)?;

    Ok(NonrootCyclesCapabilityResponseV1 { response })
}
