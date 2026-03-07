use crate::{
    cdk::types::Principal,
    dto::{
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            DelegatedGrant, DelegatedGrantProof, PROOF_VERSION_V1, RootCapabilityEnvelopeV1,
            RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{Request, RootRequestMetadata},
    },
    log,
    log::Topic,
    ops::{
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
        storage::{auth::DelegationStateOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::rpc::request::handler::RootResponseWorkflow,
};
use candid::encode_one;
use sha2::{Digest, Sha256};

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
    if service != CapabilityService::Root {
        return Err(Error::invalid(
            "capability envelope service must be Root for root dispatch",
        ));
    }

    if capability_version != CAPABILITY_VERSION_V1 {
        return Err(Error::invalid(format!(
            "unsupported capability_version: {capability_version}",
        )));
    }

    match proof {
        CapabilityProof::Structural => Ok(()),
        CapabilityProof::RoleAttestation(proof) => {
            if proof.proof_version != PROOF_VERSION_V1 {
                return Err(Error::invalid(format!(
                    "unsupported role attestation proof_version: {}",
                    proof.proof_version
                )));
            }
            Ok(())
        }
        CapabilityProof::DelegatedGrant(proof) => {
            if proof.proof_version != PROOF_VERSION_V1 {
                return Err(Error::invalid(format!(
                    "unsupported delegated grant proof_version: {}",
                    proof.proof_version
                )));
            }
            Ok(())
        }
    }
}

async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    let target_canister = IcOps::canister_self();

    match proof {
        CapabilityProof::Structural => verify_root_structural_proof(capability),
        CapabilityProof::RoleAttestation(proof) => {
            verify_capability_hash_binding(
                target_canister,
                capability_version,
                capability,
                proof.capability_hash,
            )?;

            crate::api::auth::DelegationApi::verify_role_attestation(&proof.attestation, 0).await
        }
        CapabilityProof::DelegatedGrant(proof) => {
            verify_capability_hash_binding(
                target_canister,
                capability_version,
                capability,
                proof.capability_hash,
            )?;
            verify_delegated_grant_hash_binding(proof)?;
            verify_root_delegated_grant_proof(
                capability,
                proof,
                IcOps::msg_caller(),
                target_canister,
                IcOps::now_secs(),
            )
        }
    }
}

fn verify_root_structural_proof(capability: &Request) -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if SubnetRegistryOps::get(caller).is_none() {
        return Err(Error::forbidden(
            "structural proof requires caller to be registered in subnet registry",
        ));
    }

    match capability {
        Request::Cycles(_) => Ok(()),
        Request::UpgradeCanister(req) => {
            let target = SubnetRegistryOps::get(req.canister_pid).ok_or_else(|| {
                Error::forbidden("structural proof requires registered upgrade target")
            })?;
            if target.parent_pid != Some(caller) {
                return Err(Error::forbidden(
                    "structural proof requires upgrade target to be a direct child of caller",
                ));
            }
            Ok(())
        }
        _ => Err(Error::forbidden(
            "structural proof is only supported for root cycles and upgrade capabilities",
        )),
    }
}

fn verify_capability_hash_binding(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
    capability_hash: [u8; 32],
) -> Result<(), Error> {
    let expected = root_capability_hash(target_canister, capability_version, capability)?;
    if capability_hash != expected {
        return Err(Error::invalid(
            "capability_hash does not match capability payload",
        ));
    }

    Ok(())
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
    if proof.grant.capability_hash != proof.capability_hash {
        return Err(Error::invalid(
            "delegated grant capability_hash does not match proof capability_hash",
        ));
    }

    Ok(())
}

fn verify_root_delegated_grant_proof(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    verify_root_delegated_grant_claims(capability, proof, caller, target_canister, now_secs)?;
    verify_root_delegated_grant_signature(&proof.grant, &proof.grant_sig)
}

fn verify_root_delegated_grant_claims(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    if proof.key_id != DELEGATED_GRANT_KEY_ID_V1 {
        return Err(Error::invalid(format!(
            "unsupported delegated grant key_id: {}",
            proof.key_id
        )));
    }

    let grant = &proof.grant;
    if grant.issuer != target_canister {
        return Err(Error::forbidden(
            "delegated grant issuer must match target canister",
        ));
    }
    if grant.subject != caller {
        return Err(Error::forbidden(
            "delegated grant subject must match caller",
        ));
    }
    if !grant.audience.contains(&target_canister) {
        return Err(Error::forbidden(
            "delegated grant audience must include target canister",
        ));
    }
    if grant.scope.service != CapabilityService::Root {
        return Err(Error::forbidden(
            "delegated grant scope service must be Root",
        ));
    }
    let expected_family = root_capability_family(capability);
    if grant.scope.capability_family != expected_family {
        return Err(Error::forbidden(format!(
            "delegated grant scope capability_family '{}' does not match '{}'",
            grant.scope.capability_family, expected_family
        )));
    }
    if grant.quota == 0 {
        return Err(Error::invalid(
            "delegated grant quota must be greater than zero",
        ));
    }
    if grant.expires_at <= grant.issued_at {
        return Err(Error::invalid(
            "delegated grant expires_at must be greater than issued_at",
        ));
    }
    if now_secs < grant.issued_at {
        return Err(Error::forbidden(
            "delegated grant is not valid yet for current time",
        ));
    }
    if now_secs > grant.expires_at {
        return Err(Error::forbidden("delegated grant has expired"));
    }

    Ok(())
}

fn verify_root_delegated_grant_signature(
    grant: &DelegatedGrant,
    signature: &[u8],
) -> Result<(), Error> {
    if signature.is_empty() {
        return Err(Error::forbidden("delegated grant signature is required"));
    }

    let root_public_key = DelegationStateOps::root_public_key()
        .ok_or_else(|| Error::forbidden("delegated grant root public key unavailable"))?;
    let grant_hash = delegated_grant_hash(grant)?;
    EcdsaOps::verify_signature(&root_public_key, grant_hash, signature)
        .map_err(|err| Error::forbidden(format!("delegated grant signature invalid: {err}")))?;

    Ok(())
}

const fn root_capability_family(capability: &Request) -> &'static str {
    match capability {
        Request::CreateCanister(_) => "Provision",
        Request::UpgradeCanister(_) => "Upgrade",
        Request::Cycles(_) => "MintCycles",
        Request::IssueDelegation(_) => "IssueDelegation",
        Request::IssueRoleAttestation(_) => "IssueRoleAttestation",
    }
}

fn delegated_grant_hash(grant: &DelegatedGrant) -> Result<[u8; 32], Error> {
    let payload = encode_one(grant)
        .map_err(|err| Error::internal(format!("failed to encode delegated grant: {err}")))?;
    let mut hasher = Sha256::new();
    hasher.update(DELEGATED_GRANT_SIGNING_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn root_capability_hash(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
) -> Result<[u8; 32], Error> {
    let canonical = strip_request_metadata(capability.clone());
    let payload = encode_one(&(
        target_canister,
        CapabilityService::Root,
        capability_version,
        canonical,
    ))
    .map_err(|err| Error::internal(format!("failed to encode capability payload: {err}")))?;
    let mut hasher = Sha256::new();
    hasher.update(CAPABILITY_HASH_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn with_root_request_metadata(request: Request, metadata: RootRequestMetadata) -> Request {
    match request {
        Request::CreateCanister(mut req) => {
            req.metadata = Some(metadata);
            Request::CreateCanister(req)
        }
        Request::UpgradeCanister(mut req) => {
            req.metadata = Some(metadata);
            Request::UpgradeCanister(req)
        }
        Request::Cycles(mut req) => {
            req.metadata = Some(metadata);
            Request::Cycles(req)
        }
        Request::IssueDelegation(mut req) => {
            req.metadata = Some(metadata);
            Request::IssueDelegation(req)
        }
        Request::IssueRoleAttestation(mut req) => {
            req.metadata = Some(metadata);
            Request::IssueRoleAttestation(req)
        }
    }
}

fn strip_request_metadata(request: Request) -> Request {
    match request {
        Request::CreateCanister(mut req) => {
            req.metadata = None;
            Request::CreateCanister(req)
        }
        Request::UpgradeCanister(mut req) => {
            req.metadata = None;
            Request::UpgradeCanister(req)
        }
        Request::Cycles(mut req) => {
            req.metadata = None;
            Request::Cycles(req)
        }
        Request::IssueDelegation(mut req) => {
            req.metadata = None;
            Request::IssueDelegation(req)
        }
        Request::IssueRoleAttestation(mut req) => {
            req.metadata = None;
            Request::IssueRoleAttestation(req)
        }
    }
}

fn project_replay_metadata(
    metadata: CapabilityRequestMetadata,
    now_secs: u64,
) -> Result<RootRequestMetadata, Error> {
    if metadata.ttl_seconds == 0 {
        return Err(Error::invalid(
            "capability metadata ttl_seconds must be greater than zero",
        ));
    }

    if metadata.issued_at > now_secs.saturating_add(MAX_CAPABILITY_CLOCK_SKEW_SECONDS) {
        return Err(Error::invalid(
            "capability metadata issued_at is too far in the future",
        ));
    }

    let expires_at = metadata
        .issued_at
        .checked_add(u64::from(metadata.ttl_seconds))
        .ok_or_else(|| Error::invalid("capability metadata expiry overflow"))?;
    if now_secs > expires_at {
        return Err(Error::conflict("capability metadata has expired"));
    }

    Ok(RootRequestMetadata {
        request_id: replay_request_id(metadata.request_id, metadata.nonce),
        ttl_seconds: u64::from(metadata.ttl_seconds),
    })
}

fn replay_request_id(request_id: [u8; 16], nonce: [u8; 16]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(REPLAY_REQUEST_ID_DOMAIN_V1);
    hasher.update(request_id);
    hasher.update(nonce);
    hasher.finalize().into()
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{
        auth::{RoleAttestation, SignedRoleAttestation},
        capability::DelegatedGrantScope,
        rpc::{CyclesRequest, RootRequestMetadata},
    };
    use k256::ecdsa::{Signature, SigningKey, signature::hazmat::PrehashSigner};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_request(cycles: u128) -> Request {
        Request::Cycles(CyclesRequest {
            cycles,
            metadata: None,
        })
    }

    fn sample_metadata(
        request_id: u8,
        nonce: u8,
        issued_at: u64,
        ttl_seconds: u32,
    ) -> CapabilityRequestMetadata {
        CapabilityRequestMetadata {
            request_id: [request_id; 16],
            nonce: [nonce; 16],
            issued_at,
            ttl_seconds,
        }
    }

    #[test]
    fn root_capability_hash_changes_with_payload() {
        let hash_a =
            root_capability_hash(p(1), CAPABILITY_VERSION_V1, &sample_request(10)).expect("hash a");
        let hash_b =
            root_capability_hash(p(1), CAPABILITY_VERSION_V1, &sample_request(11)).expect("hash b");
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn root_capability_hash_binds_target_canister() {
        let req = sample_request(10);
        let hash_a = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req).expect("hash a");
        let hash_b = root_capability_hash(p(2), CAPABILITY_VERSION_V1, &req).expect("hash b");
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn root_capability_hash_binds_capability_version() {
        let req = sample_request(10);
        let hash_a = root_capability_hash(p(1), 1, &req).expect("hash a");
        let hash_b = root_capability_hash(p(1), 2, &req).expect("hash b");
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn root_capability_hash_ignores_request_metadata() {
        let req_a = Request::Cycles(CyclesRequest {
            cycles: 10,
            metadata: Some(RootRequestMetadata {
                request_id: [1u8; 32],
                ttl_seconds: 60,
            }),
        });
        let req_b = Request::Cycles(CyclesRequest {
            cycles: 10,
            metadata: Some(RootRequestMetadata {
                request_id: [2u8; 32],
                ttl_seconds: 120,
            }),
        });

        let hash_a = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req_a).expect("hash a");
        let hash_b = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req_b).expect("hash b");
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn project_replay_metadata_rejects_expired_metadata() {
        let err = project_replay_metadata(sample_metadata(1, 2, 900, 50), 1_000)
            .expect_err("expired metadata must fail");
        assert!(err.message.contains("expired"));
    }

    #[test]
    fn project_replay_metadata_rejects_future_metadata_beyond_skew() {
        let err = project_replay_metadata(sample_metadata(1, 2, 1_031, 60), 1_000)
            .expect_err("future metadata must fail");
        assert!(err.message.contains("future"));
    }

    #[test]
    fn project_replay_metadata_binds_nonce_into_request_id() {
        let a = project_replay_metadata(sample_metadata(3, 1, 1_000, 60), 1_000).expect("a");
        let b = project_replay_metadata(sample_metadata(3, 2, 1_000, 60), 1_000).expect("b");
        assert_ne!(a.request_id, b.request_id);
    }

    #[test]
    fn with_root_request_metadata_overrides_existing_metadata() {
        let request = Request::Cycles(CyclesRequest {
            cycles: 10,
            metadata: Some(RootRequestMetadata {
                request_id: [7u8; 32],
                ttl_seconds: 10,
            }),
        });
        let metadata = RootRequestMetadata {
            request_id: [9u8; 32],
            ttl_seconds: 60,
        };

        let updated = with_root_request_metadata(request, metadata);
        match updated {
            Request::Cycles(req) => assert_eq!(req.metadata, Some(metadata)),
            _ => panic!("expected cycles request"),
        }
    }

    fn sample_signed_attestation() -> SignedRoleAttestation {
        SignedRoleAttestation {
            payload: RoleAttestation {
                subject: p(1),
                role: crate::ids::CanisterRole::ROOT,
                subnet_id: None,
                audience: Some(p(2)),
                issued_at: 1_000,
                expires_at: 2_000,
                epoch: 1,
            },
            signature: vec![],
            key_id: 1,
        }
    }

    fn sample_delegated_grant_proof(
        capability: &Request,
        caller: Principal,
        target_canister: Principal,
        now_secs: u64,
    ) -> DelegatedGrantProof {
        let capability_hash =
            root_capability_hash(target_canister, CAPABILITY_VERSION_V1, capability).expect("hash");
        DelegatedGrantProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash,
            grant: DelegatedGrant {
                issuer: target_canister,
                subject: caller,
                audience: vec![target_canister],
                scope: DelegatedGrantScope {
                    service: CapabilityService::Root,
                    capability_family: root_capability_family(capability).to_string(),
                },
                capability_hash,
                quota: 1,
                issued_at: now_secs.saturating_sub(10),
                expires_at: now_secs.saturating_add(10),
                epoch: 0,
            },
            grant_sig: vec![1],
            key_id: DELEGATED_GRANT_KEY_ID_V1,
        }
    }

    fn sign_delegated_grant(seed: u8, grant: &DelegatedGrant) -> (Vec<u8>, Vec<u8>) {
        let signing_key = SigningKey::from_bytes((&[seed; 32]).into()).expect("signing key");
        let signature: Signature = signing_key
            .sign_prehash(&delegated_grant_hash(grant).expect("hash"))
            .expect("prehash signature");
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec();
        (public_key, signature.to_bytes().to_vec())
    }

    #[test]
    fn validate_root_capability_envelope_rejects_service_mismatch() {
        let err = validate_root_capability_envelope(
            CapabilityService::Cycles,
            CAPABILITY_VERSION_V1,
            &CapabilityProof::Structural,
        )
        .expect_err("service mismatch must fail");
        assert!(err.message.contains("service"));
    }

    #[test]
    fn validate_root_capability_envelope_rejects_capability_version_mismatch() {
        let err = validate_root_capability_envelope(
            CapabilityService::Root,
            CAPABILITY_VERSION_V1 + 1,
            &CapabilityProof::Structural,
        )
        .expect_err("unsupported capability version must fail");
        assert!(err.message.contains("capability_version"));
    }

    #[test]
    fn validate_root_capability_envelope_rejects_role_attestation_proof_version_mismatch() {
        let err = validate_root_capability_envelope(
            CapabilityService::Root,
            CAPABILITY_VERSION_V1,
            &CapabilityProof::RoleAttestation(crate::dto::capability::RoleAttestationProof {
                proof_version: PROOF_VERSION_V1 + 1,
                capability_hash: [0u8; 32],
                attestation: sample_signed_attestation(),
            }),
        )
        .expect_err("unsupported role proof version must fail");
        assert!(err.message.contains("proof_version"));
    }

    #[test]
    fn verify_capability_hash_binding_rejects_mismatch() {
        let err = verify_capability_hash_binding(
            p(1),
            CAPABILITY_VERSION_V1,
            &sample_request(10),
            [0u8; 32],
        )
        .expect_err("mismatched hash must fail");
        assert!(err.message.contains("capability_hash"));
    }

    #[test]
    fn verify_capability_hash_binding_accepts_match() {
        let request = sample_request(10);
        let hash = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &request).expect("hash");
        verify_capability_hash_binding(p(1), CAPABILITY_VERSION_V1, &request, hash)
            .expect("matching hash must verify");
    }

    #[test]
    fn verify_delegated_grant_hash_binding_rejects_mismatch() {
        let proof = DelegatedGrantProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: [1u8; 32],
            grant: crate::dto::capability::DelegatedGrant {
                issuer: p(1),
                subject: p(2),
                audience: vec![p(3)],
                scope: crate::dto::capability::DelegatedGrantScope {
                    service: CapabilityService::Root,
                    capability_family: "root".to_string(),
                },
                capability_hash: [2u8; 32],
                quota: 1,
                issued_at: 1,
                expires_at: 2,
                epoch: 0,
            },
            grant_sig: vec![],
            key_id: 1,
        };

        let err = verify_delegated_grant_hash_binding(&proof)
            .expect_err("mismatched delegated grant hash must fail");
        assert!(err.message.contains("capability_hash"));
    }

    #[test]
    fn delegated_grant_hash_changes_with_payload() {
        let grant_a = DelegatedGrant {
            issuer: p(1),
            subject: p(2),
            audience: vec![p(1)],
            scope: DelegatedGrantScope {
                service: CapabilityService::Root,
                capability_family: "MintCycles".to_string(),
            },
            capability_hash: [1u8; 32],
            quota: 1,
            issued_at: 10,
            expires_at: 20,
            epoch: 0,
        };
        let mut grant_b = grant_a.clone();
        grant_b.quota = 2;

        let hash_a = delegated_grant_hash(&grant_a).expect("hash a");
        let hash_b = delegated_grant_hash(&grant_b).expect("hash b");
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn verify_root_delegated_grant_claims_accepts_matching_scope() {
        let now_secs = 100;
        let caller = p(2);
        let target_canister = p(1);
        let capability = sample_request(10);
        let proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);

        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect("matching delegated grant claims must verify");
    }

    #[test]
    fn verify_root_delegated_grant_claims_rejects_subject_mismatch() {
        let now_secs = 100;
        let caller = p(2);
        let target_canister = p(1);
        let capability = sample_request(10);
        let mut proof =
            sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
        proof.grant.subject = p(3);

        let err = verify_root_delegated_grant_claims(
            &capability,
            &proof,
            caller,
            target_canister,
            now_secs,
        )
        .expect_err("subject mismatch must fail");
        assert!(err.message.contains("subject"));
    }

    #[test]
    fn verify_root_delegated_grant_claims_rejects_scope_family_mismatch() {
        let now_secs = 100;
        let caller = p(2);
        let target_canister = p(1);
        let capability = sample_request(10);
        let mut proof =
            sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
        proof.grant.scope.capability_family = "Upgrade".to_string();

        let err = verify_root_delegated_grant_claims(
            &capability,
            &proof,
            caller,
            target_canister,
            now_secs,
        )
        .expect_err("scope family mismatch must fail");
        assert!(err.message.contains("capability_family"));
    }

    #[test]
    fn verify_root_delegated_grant_claims_rejects_key_id_mismatch() {
        let now_secs = 100;
        let caller = p(2);
        let target_canister = p(1);
        let capability = sample_request(10);
        let mut proof =
            sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
        proof.key_id = DELEGATED_GRANT_KEY_ID_V1 + 1;

        let err = verify_root_delegated_grant_claims(
            &capability,
            &proof,
            caller,
            target_canister,
            now_secs,
        )
        .expect_err("unsupported key_id must fail");
        assert!(err.message.contains("key_id"));
    }

    #[test]
    fn verify_root_delegated_grant_signature_accepts_valid_signature() {
        let capability = sample_request(10);
        let proof = sample_delegated_grant_proof(&capability, p(2), p(1), 100);
        let (public_key, signature) = sign_delegated_grant(7, &proof.grant);
        DelegationStateOps::set_root_public_key(public_key);

        verify_root_delegated_grant_signature(&proof.grant, &signature)
            .expect("valid delegated grant signature must verify");
    }

    #[test]
    fn verify_root_delegated_grant_signature_rejects_invalid_signature() {
        let capability = sample_request(10);
        let proof = sample_delegated_grant_proof(&capability, p(2), p(1), 100);
        let (public_key, _signature) = sign_delegated_grant(7, &proof.grant);
        let (_, wrong_signature) = sign_delegated_grant(8, &proof.grant);
        DelegationStateOps::set_root_public_key(public_key);

        let err = verify_root_delegated_grant_signature(&proof.grant, &wrong_signature)
            .expect_err("invalid signature must fail");
        assert!(err.message.contains("signature invalid"));
    }
}
