use super::{
    capability_proof_mode_label, capability_proof_mode_metric_key, project_replay_metadata,
    root_capability_family, root_capability_metric_key, validate_nonroot_cycles_envelope,
    verify_nonroot_cycles_proof, with_root_request_metadata,
};
use crate::{
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::Error,
        rpc::{Request, Response},
    },
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
    },
    workflow::rpc::request::handler::NonrootCyclesCapabilityWorkflow,
};

/// response_capability_v1_nonroot
///
/// Execute the non-root structural cycles capability path.
pub(super) async fn response_capability_v1_nonroot(
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
    let proof_mode = capability_proof_mode_metric_key(&proof);
    if let Err(err) =
        validate_nonroot_cycles_envelope(service, capability_version, &capability, &proof)
    {
        RootCapabilityMetrics::record_envelope(
            capability_key,
            RootCapabilityMetricOutcome::Rejected,
            proof_mode,
        );
        log!(
            Topic::Rpc,
            Warn,
            "non-root capability envelope rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            root_capability_family(&capability),
            IcOps::msg_caller(),
            service,
            capability_version,
            capability_proof_mode_label(&proof),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record_envelope(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode,
    );

    if let Err(err) = verify_nonroot_cycles_proof(&capability) {
        RootCapabilityMetrics::record_proof(
            capability_key,
            RootCapabilityMetricOutcome::Rejected,
            proof_mode,
        );
        log!(
            Topic::Rpc,
            Warn,
            "non-root capability proof rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            root_capability_family(&capability),
            IcOps::msg_caller(),
            service,
            capability_version,
            capability_proof_mode_label(&proof),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record_proof(
        capability_key,
        RootCapabilityMetricOutcome::Accepted,
        proof_mode,
    );

    let replay_metadata = project_replay_metadata(metadata, IcOps::now_secs())?;
    let capability = with_root_request_metadata(capability, replay_metadata);
    let Request::Cycles(request) = capability else {
        return Err(Error::invariant(
            "non-root capability endpoint only supports cycles requests",
        ));
    };
    let response = NonrootCyclesCapabilityWorkflow::response_replay_first(request)
        .await
        .map_err(Error::from)?;

    Ok(RootCapabilityResponseV1 {
        response: Response::Cycles(response),
    })
}
