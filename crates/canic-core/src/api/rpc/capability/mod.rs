use crate::{
    cdk::types::Principal,
    dto::{
        capability::{
            CapabilityProof, CapabilityRequestMetadata, CapabilityService, DelegatedGrantProof,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{Request, RootRequestMetadata},
    },
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
    },
    workflow::rpc::request::handler::RootResponseWorkflow,
};

mod envelope;
mod grant;
mod hash;
mod proof;
mod replay;

#[cfg(test)]
mod tests;

const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";
const DELEGATED_GRANT_SIGNING_DOMAIN_V1: &[u8] = b"CANIC_DELEGATED_GRANT_V1";
const REPLAY_REQUEST_ID_DOMAIN_V1: &[u8] = b"CANIC_REPLAY_REQUEST_ID_V1";
const MAX_CAPABILITY_CLOCK_SKEW_SECONDS: u64 = 30;
const DELEGATED_GRANT_KEY_ID_V1: u32 = 1;

pub(super) async fn response_capability_v1(
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
    if let Err(err) = validate_root_capability_envelope(service, capability_version, &proof) {
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::EnvelopeRejected);
        log!(
            Topic::Rpc,
            Warn,
            "root capability envelope rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            root_capability_family(&capability),
            IcOps::msg_caller(),
            service,
            capability_version,
            capability_proof_mode_label(&proof),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::EnvelopeValidated);

    if let Err(err) = verify_root_capability_proof(&capability, capability_version, &proof).await {
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ProofRejected);
        log!(
            Topic::Rpc,
            Warn,
            "root capability proof rejected (capability={}, caller={}, service={:?}, capability_version={}, proof_mode={}): {}",
            root_capability_family(&capability),
            IcOps::msg_caller(),
            service,
            capability_version,
            capability_proof_mode_label(&proof),
            err
        );
        return Err(err);
    }
    RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ProofVerified);

    let replay_metadata = project_replay_metadata(metadata, IcOps::now_secs())?;
    let capability = with_root_request_metadata(capability, replay_metadata);
    let response = RootResponseWorkflow::response_replay_first(capability)
        .await
        .map_err(Error::from)?;

    Ok(RootCapabilityResponseV1 { response })
}

fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    envelope::validate_root_capability_envelope(service, capability_version, proof)
}

async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    proof::verify_root_capability_proof(capability, capability_version, proof).await
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

const fn root_capability_metric_key(capability: &Request) -> RootCapabilityMetricKey {
    match capability {
        Request::CreateCanister(_) => RootCapabilityMetricKey::Provision,
        Request::UpgradeCanister(_) => RootCapabilityMetricKey::Upgrade,
        Request::Cycles(_) => RootCapabilityMetricKey::MintCycles,
        Request::IssueDelegation(_) => RootCapabilityMetricKey::IssueDelegation,
        Request::IssueRoleAttestation(_) => RootCapabilityMetricKey::IssueRoleAttestation,
    }
}

const fn capability_proof_mode_label(proof: &CapabilityProof) -> &'static str {
    match proof {
        CapabilityProof::Structural => "Structural",
        CapabilityProof::RoleAttestation(_) => "RoleAttestation",
        CapabilityProof::DelegatedGrant(_) => "DelegatedGrant",
    }
}

fn verify_delegated_grant_hash_binding(proof: &DelegatedGrantProof) -> Result<(), Error> {
    grant::verify_delegated_grant_hash_binding(proof)
}

fn verify_root_delegated_grant_proof(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    grant::verify_root_delegated_grant_proof(capability, proof, caller, target_canister, now_secs)
}

#[cfg(test)]
fn verify_root_delegated_grant_claims(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    grant::verify_root_delegated_grant_claims(capability, proof, caller, target_canister, now_secs)
}

#[cfg(test)]
fn verify_root_delegated_grant_signature(
    grant: &crate::dto::capability::DelegatedGrant,
    signature: &[u8],
) -> Result<(), Error> {
    grant::verify_root_delegated_grant_signature(grant, signature)
}

const fn root_capability_family(capability: &Request) -> &'static str {
    grant::root_capability_family(capability)
}

#[cfg(test)]
fn delegated_grant_hash(grant: &crate::dto::capability::DelegatedGrant) -> Result<[u8; 32], Error> {
    grant::delegated_grant_hash(grant)
}

fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    hash::root_capability_hash(target_canister, capability_version, capability)
}

fn with_root_request_metadata(request: Request, metadata: RootRequestMetadata) -> Request {
    replay::with_root_request_metadata(request, metadata)
}

fn project_replay_metadata(
    metadata: CapabilityRequestMetadata,
    now_secs: u64,
) -> Result<RootRequestMetadata, Error> {
    replay::project_replay_metadata(metadata, now_secs)
}
