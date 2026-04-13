use crate::{
    cdk::types::Principal,
    dto::{
        capability::{
            CapabilityProof, CapabilityRequestMetadata, CapabilityService, DelegatedGrantProof,
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
        },
        error::Error,
        rpc::{Request, RequestFamily, RootRequestMetadata},
    },
    ops::runtime::metrics::root_capability::{
        RootCapabilityMetricKey, RootCapabilityMetricProofMode,
    },
};

mod envelope;
mod grant;
mod hash;
mod nonroot;
mod proof;
mod replay;
mod root;
mod verifier;

#[cfg(test)]
mod tests;

const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";
const DELEGATED_GRANT_SIGNING_DOMAIN_V1: &[u8] = b"CANIC_DELEGATED_GRANT_V1";
const REPLAY_REQUEST_ID_DOMAIN_V1: &[u8] = b"CANIC_REPLAY_REQUEST_ID_V1";
const MAX_CAPABILITY_CLOCK_SKEW_SECONDS: u64 = 30;
const DELEGATED_GRANT_KEY_ID_V1: u32 = 1;

pub(super) async fn response_capability_v1_nonroot(
    envelope: NonrootCyclesCapabilityEnvelopeV1,
) -> Result<NonrootCyclesCapabilityResponseV1, Error> {
    nonroot::response_capability_v1_nonroot(envelope).await
}

pub(super) async fn response_capability_v1_root(
    envelope: crate::dto::capability::RootCapabilityEnvelopeV1,
) -> Result<crate::dto::capability::RootCapabilityResponseV1, Error> {
    root::response_capability_v1_root(envelope).await
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
    envelope::validate_root_capability_envelope(service, capability_version, proof)?;

    if !matches!(proof, CapabilityProof::Structural) {
        return Err(Error::forbidden(
            "non-root capability endpoint only supports structural proof mode",
        ));
    }

    Ok(())
}

async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: &CapabilityProof,
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

const fn root_capability_metric_key(capability: &Request) -> RootCapabilityMetricKey {
    match capability.family() {
        RequestFamily::Provision => RootCapabilityMetricKey::Provision,
        RequestFamily::Upgrade => RootCapabilityMetricKey::Upgrade,
        RequestFamily::RecycleCanister => RootCapabilityMetricKey::RecycleCanister,
        RequestFamily::RequestCycles => RootCapabilityMetricKey::RequestCycles,
        RequestFamily::IssueDelegation => RootCapabilityMetricKey::IssueDelegation,
        RequestFamily::IssueRoleAttestation => RootCapabilityMetricKey::IssueRoleAttestation,
    }
}

const fn capability_proof_mode_label(proof: &CapabilityProof) -> &'static str {
    match proof {
        CapabilityProof::Structural => "Structural",
        CapabilityProof::RoleAttestation(_) => "RoleAttestation",
        CapabilityProof::DelegatedGrant(_) => "DelegatedGrant",
    }
}

const fn capability_proof_mode_metric_key(
    proof: &CapabilityProof,
) -> RootCapabilityMetricProofMode {
    match proof {
        CapabilityProof::Structural => RootCapabilityMetricProofMode::Structural,
        CapabilityProof::RoleAttestation(_) => RootCapabilityMetricProofMode::RoleAttestation,
        CapabilityProof::DelegatedGrant(_) => RootCapabilityMetricProofMode::DelegatedGrant,
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

pub fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    hash::root_capability_hash(target_canister, capability_version, capability)
}

const fn with_root_request_metadata(request: Request, metadata: RootRequestMetadata) -> Request {
    replay::with_root_request_metadata(request, metadata)
}

fn project_replay_metadata(
    metadata: CapabilityRequestMetadata,
    now_secs: u64,
) -> Result<RootRequestMetadata, Error> {
    replay::project_replay_metadata(metadata, now_secs)
}
