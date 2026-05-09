use crate::{
    cdk::types::Principal,
    dto::{
        capability::{
            CapabilityProof, CapabilityProofBlob, CapabilityRequestMetadata, CapabilityService,
            DelegatedGrantProof, NonrootCyclesCapabilityEnvelopeV1,
            NonrootCyclesCapabilityResponseV1, PROOF_VERSION_V1,
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

/// RootCapabilityProofMode
///
/// Canonical classification for capability proof logging and metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootCapabilityProofMode {
    Structural,
    RoleAttestation,
    DelegatedGrant,
}

impl RootCapabilityProofMode {
    /// Classify a raw proof without validating its payload header.
    const fn from_proof(proof: &CapabilityProof) -> Self {
        match proof {
            CapabilityProof::Structural => Self::Structural,
            CapabilityProof::RoleAttestation(_) => Self::RoleAttestation,
            CapabilityProof::DelegatedGrant(_) => Self::DelegatedGrant,
        }
    }

    /// Human-readable label used in capability logs.
    const fn label(self) -> &'static str {
        match self {
            Self::Structural => "Structural",
            Self::RoleAttestation => "RoleAttestation",
            Self::DelegatedGrant => "DelegatedGrant",
        }
    }

    /// Metrics dimension matching the canonical proof classification.
    const fn metric_key(self) -> RootCapabilityMetricProofMode {
        match self {
            Self::Structural => RootCapabilityMetricProofMode::Structural,
            Self::RoleAttestation => RootCapabilityMetricProofMode::RoleAttestation,
            Self::DelegatedGrant => RootCapabilityMetricProofMode::DelegatedGrant,
        }
    }
}

/// RootCapabilityProof
///
/// Validated proof view used after envelope checks and before verification.
#[derive(Clone, Copy, Debug)]
pub(super) enum RootCapabilityProof<'a> {
    Structural,
    RoleAttestation(&'a CapabilityProofBlob),
    DelegatedGrant(&'a CapabilityProofBlob),
}

impl<'a> RootCapabilityProof<'a> {
    /// Validate the proof wire header and expose the typed proof view.
    fn validate(proof: &'a CapabilityProof) -> Result<Self, Error> {
        match proof {
            CapabilityProof::Structural => Ok(Self::Structural),
            CapabilityProof::RoleAttestation(proof) => {
                if proof.proof_version != PROOF_VERSION_V1 {
                    return Err(Error::invalid(format!(
                        "unsupported role attestation proof_version: {}",
                        proof.proof_version
                    )));
                }
                Ok(Self::RoleAttestation(proof))
            }
            CapabilityProof::DelegatedGrant(proof) => {
                if proof.proof_version != PROOF_VERSION_V1 {
                    return Err(Error::invalid(format!(
                        "unsupported delegated grant proof_version: {}",
                        proof.proof_version
                    )));
                }
                Ok(Self::DelegatedGrant(proof))
            }
        }
    }

    /// Return the canonical proof mode for this validated proof.
    const fn mode(self) -> RootCapabilityProofMode {
        match self {
            Self::Structural => RootCapabilityProofMode::Structural,
            Self::RoleAttestation(_) => RootCapabilityProofMode::RoleAttestation,
            Self::DelegatedGrant(_) => RootCapabilityProofMode::DelegatedGrant,
        }
    }
}

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
) -> Result<RootCapabilityProof<'_>, Error> {
    envelope::validate_root_capability_envelope(service, capability_version, proof)
}

fn validate_nonroot_cycles_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<RootCapabilityProof<'_>, Error> {
    let proof = envelope::validate_root_capability_envelope(service, capability_version, proof)?;

    if proof.mode() != RootCapabilityProofMode::Structural {
        return Err(Error::forbidden(
            "non-root capability endpoint only supports structural proof mode",
        ));
    }

    Ok(proof)
}

async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: RootCapabilityProof<'_>,
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
        RequestFamily::IssueRoleAttestation => RootCapabilityMetricKey::IssueRoleAttestation,
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
