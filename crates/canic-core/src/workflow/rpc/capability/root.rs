use super::{
    RootCapabilityProofMode, project_replay_metadata, root_capability_family,
    root_capability_metric_key, validate_root_capability_envelope, verify_root_capability_proof,
    with_root_request_metadata,
};
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
    workflow::rpc::request::handler::RootResponseWorkflow,
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

    let capability_key = root_capability_metric_key(&capability);
    let proof_mode = RootCapabilityProofMode::from_proof(&proof);
    let validated_proof = match validate_root_capability_envelope(
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
                "root capability envelope rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
                root_capability_family(&capability),
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

    if let Err(err) =
        verify_root_capability_proof(&capability, capability_version, validated_proof).await
    {
        RootCapabilityMetrics::record_proof(
            capability_key,
            RootCapabilityMetricOutcome::Rejected,
            proof_mode.metric_key(),
        );
        log!(
            Topic::Rpc,
            Warn,
            "root capability proof rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            root_capability_family(&capability),
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
    let capability = with_root_request_metadata(capability, replay_metadata);
    let response = RootResponseWorkflow::response_replay_first(capability)
        .await
        .map_err(Error::from)?;

    Ok(RootCapabilityResponseV1 { response })
}
