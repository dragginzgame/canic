use super::*;
use crate::dto::auth::{
    AttestationKeyStatus, DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
};
use k256::ecdsa::{SigningKey, signature::hazmat::PrehashSigner};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn sample_attestation(epoch: u64) -> RoleAttestation {
    RoleAttestation {
        subject: p(1),
        role: CanisterRole::new("app"),
        subnet_id: Some(p(2)),
        audience: Some(p(3)),
        issued_at: 100,
        expires_at: 200,
        epoch,
    }
}

fn sample_proof(shard_pid: Principal, issued_at: u64) -> DelegationProof {
    DelegationProof {
        cert: DelegationCert {
            root_pid: p(42),
            shard_pid,
            issued_at,
            expires_at: issued_at + 120,
            scopes: vec!["verify".to_string()],
            aud: vec![p(3)],
        },
        cert_sig: vec![shard_pid.as_slice()[0], issued_at.to_le_bytes()[0]],
    }
}

fn sample_claims() -> DelegatedTokenClaims {
    DelegatedTokenClaims {
        sub: p(9),
        shard_pid: p(2),
        scopes: vec!["verify".to_string()],
        aud: vec![p(3)],
        iat: 100,
        exp: 120,
        ext: None,
    }
}

fn sample_claims_for(
    shard_pid: Principal,
    audience: Principal,
    iat: u64,
    exp: u64,
) -> DelegatedTokenClaims {
    DelegatedTokenClaims {
        sub: p(9),
        shard_pid,
        scopes: vec!["verify".to_string()],
        aud: vec![audience],
        iat,
        exp,
        ext: None,
    }
}

fn sign_prehash(seed: u8, hash: [u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let signing_key = SigningKey::from_bytes((&[seed; 32]).into()).expect("signing key");
    let signature: k256::ecdsa::Signature =
        signing_key.sign_prehash(&hash).expect("prehash signature");
    let public_key = signing_key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();
    (public_key, signature.to_bytes().to_vec())
}

fn signed_token_for(
    shard_pid: Principal,
    audience: Principal,
    cert_issued_at: u64,
    token_iat: u64,
    token_exp: u64,
    root_seed: u8,
    shard_seed: u8,
) -> DelegatedToken {
    let claims = sample_claims_for(shard_pid, audience, token_iat, token_exp);
    let mut proof = sample_proof(shard_pid, cert_issued_at);
    proof.cert.aud = vec![audience];

    let (root_public_key, cert_sig) = sign_prehash(root_seed, crypto::cert_hash(&proof.cert));
    let (shard_public_key, token_sig) = sign_prehash(
        shard_seed,
        crypto::token_signing_hash(&VerifiedTokenClaims::from_dto_ref(&claims), &proof.cert)
            .expect("token hash"),
    );
    proof.cert_sig = cert_sig;

    DelegationStateOps::set_root_public_key(root_public_key);
    DelegationStateOps::set_shard_public_key(shard_pid, shard_public_key);

    DelegatedToken {
        claims,
        proof,
        token_sig,
    }
}

fn signing_material(seed: u8, payload: &RoleAttestation) -> (Vec<u8>, Vec<u8>) {
    let signing_key = SigningKey::from_bytes((&[seed; 32]).into()).expect("signing key");
    let signature: k256::ecdsa::Signature = signing_key
        .sign_prehash(&role_attestation_hash(payload).expect("hash"))
        .expect("prehash signature");
    let public_key = signing_key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();
    (public_key, signature.to_bytes().to_vec())
}

#[test]
fn role_attestation_hash_changes_with_payload() {
    let hash_a = role_attestation_hash(&sample_attestation(1)).expect("hash");
    let hash_b = role_attestation_hash(&sample_attestation(2)).expect("hash");
    assert_ne!(hash_a, hash_b, "epoch must affect attestation hash");
}

#[test]
fn audience_helpers_use_set_semantics_for_principals() {
    assert!(audience::principals_subset(&[p(3), p(3)], &[p(3), p(4)]));
    assert!(!audience::principals_subset(&[p(3), p(5)], &[p(3), p(4)]));
}

#[test]
fn audience_helpers_validate_claims_against_cert() {
    let claims = VerifiedTokenClaims::from_dto(sample_claims());
    audience::validate_claims_against_cert(claims.grant(), &sample_proof(p(2), 90).cert)
        .expect("claims must fit cert");
}

#[test]
fn audience_helpers_reject_claim_outside_cert_audience() {
    let mut claims = sample_claims();
    claims.aud.push(p(8));

    let claims = VerifiedTokenClaims::from_dto(claims);
    let err = audience::validate_claims_against_cert(claims.grant(), &sample_proof(p(2), 90).cert)
        .expect_err("audience outside cert must fail");
    assert!(matches!(
        err,
        DelegationScopeError::AudienceNotAllowed { aud } if aud == p(8)
    ));
}

#[test]
fn validate_claim_invariants_rejects_expiry_before_issued() {
    let err =
        DelegatedTokenOps::validate_claim_invariants(TokenLifetime { iat: 120, exp: 110 }, 115)
            .expect_err("exp before iat must fail before deeper verification");

    assert!(matches!(
        err,
        DelegationExpiryError::TokenExpiryBeforeIssued
    ));
}

#[test]
fn verify_max_ttl_rejects_expiry_before_issued_without_underflow() {
    let err = verify::verify_max_ttl(TokenLifetime { iat: 120, exp: 110 }, 60)
        .expect_err("ttl check must fail closed on invalid lifetime");

    assert!(matches!(
        err,
        DelegationExpiryError::TokenExpiryBeforeIssued
    ));
}

#[test]
fn validate_claim_invariants_rejects_expired_lifetime() {
    let err =
        DelegatedTokenOps::validate_claim_invariants(TokenLifetime { iat: 100, exp: 110 }, 111)
            .expect_err("expired claims must fail");

    assert!(matches!(
        err,
        DelegationExpiryError::TokenExpired { exp: 110 }
    ));
}

#[test]
fn validate_claim_invariants_rejects_not_yet_valid_lifetime() {
    let err =
        DelegatedTokenOps::validate_claim_invariants(TokenLifetime { iat: 120, exp: 130 }, 119)
            .expect_err("future-issued claims must fail");

    assert!(matches!(
        err,
        DelegationExpiryError::TokenNotYetValid { iat: 120 }
    ));
}

#[test]
fn validate_claim_invariants_accepts_equal_boundary_timestamps() {
    DelegatedTokenOps::validate_claim_invariants(TokenLifetime { iat: 120, exp: 120 }, 120)
        .expect("equal issued/expiry boundary at now should remain valid");
}

#[test]
fn validate_claim_invariants_uses_explicit_zero_skew_policy() {
    let err =
        DelegatedTokenOps::validate_claim_invariants(TokenLifetime { iat: 121, exp: 130 }, 120)
            .expect_err("zero-skew policy must reject claims issued even one second in the future");

    assert!(matches!(
        err,
        DelegationExpiryError::TokenNotYetValid { iat: 121 }
    ));
}

#[test]
fn attestation_derivation_path_is_separate_from_delegation_root_path() {
    assert_ne!(
        attestation_derivation_path(),
        root_derivation_path(),
        "attestation signing must not reuse delegation root derivation path"
    );
}

#[test]
fn verify_role_attestation_claims_rejects_subject_mismatch() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, p(9), p(3), Some(p(2)), 150, 0)
        .expect_err("subject mismatch must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Scope(DelegationScopeError::AttestationSubjectMismatch { .. })
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_audience_mismatch() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(9), Some(p(2)), 150, 0)
        .expect_err("audience mismatch must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Scope(DelegationScopeError::AttestationAudienceMismatch { .. })
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_subnet_mismatch() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(8)), 150, 0)
        .expect_err("subnet mismatch must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Scope(DelegationScopeError::AttestationSubnetMismatch { .. })
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_missing_verifier_subnet() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), None, 150, 0)
        .expect_err("missing verifier subnet must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Validation(DelegationValidationError::AttestationSubnetUnavailable)
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_expired_payload() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(2)), 201, 0)
        .expect_err("expired payload must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Expiry(DelegationExpiryError::AttestationExpired { .. })
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_epoch_floor() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(2)), 150, 2)
        .expect_err("epoch floor must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Expiry(DelegationExpiryError::AttestationEpochRejected { .. })
    ));
}

#[test]
fn verify_role_attestation_cached_rejects_empty_signature() {
    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: Vec::new(),
        key_id: 1,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("empty signature must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Signature(
            DelegationSignatureError::AttestationSignatureUnavailable
        )
    ));
}

#[test]
fn verify_role_attestation_cached_reports_signature_error_before_subject_check() {
    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: Vec::new(),
        key_id: 1,
    };

    let err =
        DelegatedTokenOps::verify_role_attestation_cached(&signed, p(9), p(3), Some(p(2)), 150, 0)
            .expect_err("empty signature must fail before subject comparison");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Signature(
            DelegationSignatureError::AttestationSignatureUnavailable
        )
    ));
}

#[test]
fn verify_role_attestation_cached_reports_unknown_key_before_subject_check() {
    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1],
        key_id: 404,
    };

    let err =
        DelegatedTokenOps::verify_role_attestation_cached(&signed, p(9), p(3), Some(p(2)), 150, 0)
            .expect_err("unknown key must fail before subject comparison");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Validation(DelegationValidationError::AttestationUnknownKeyId {
            key_id: 404
        })
    ));
}

#[test]
fn verify_role_attestation_cached_rejects_key_not_yet_valid() {
    let key_id = 50;
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id,
        public_key: vec![2; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(200),
        valid_until: None,
    });

    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1],
        key_id,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("not-yet-valid key must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Expiry(DelegationExpiryError::AttestationKeyNotYetValid {
            key_id: 50,
            ..
        })
    ));
}

#[test]
fn verify_role_attestation_cached_rejects_expired_key() {
    let key_id = 51;
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id,
        public_key: vec![2; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(100),
        valid_until: Some(120),
    });

    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1],
        key_id,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("expired key must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Expiry(DelegationExpiryError::AttestationKeyExpired {
            key_id: 51,
            ..
        })
    ));
}

#[test]
fn verify_role_attestation_cached_resolves_public_key_by_key_id() {
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: 1,
        public_key: vec![3; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(100),
        valid_until: None,
    });

    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1],
        key_id: 2,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("missing key_id must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Validation(DelegationValidationError::AttestationUnknownKeyId {
            key_id: 2
        })
    ));
}

#[test]
fn verify_current_proof_accepts_matching_key_when_multiple_proofs_exist() {
    let proof_a = sample_proof(p(11), 100);
    let proof_b = sample_proof(p(12), 110);

    DelegationStateOps::upsert_proof_from_dto(proof_a.clone(), 100).expect("store proof a");
    DelegationStateOps::upsert_proof_from_dto(proof_b, 110).expect("store proof b");

    verify::verify_current_proof(&proof_a).expect("matching keyed proof must verify");
}

#[test]
fn matching_proof_lookup_distinguishes_missing_key_from_other_stored_proof() {
    let stored = sample_proof(p(21), 200);
    let missing = sample_proof(p(22), 200);

    DelegationStateOps::upsert_proof_from_dto(stored.clone(), 200).expect("store keyed proof");

    let matched = DelegationStateOps::matching_proof_dto(&stored);
    assert_eq!(matched, Some(stored), "stored proof key must resolve");

    let missing_match = DelegationStateOps::matching_proof_dto(&missing);
    assert_eq!(
        missing_match, None,
        "different proof key must resolve as miss"
    );
}

#[test]
fn verify_current_proof_accepts_same_shard_parallel_rotation_entries() {
    let old_proof = sample_proof(p(31), 300);
    let new_proof = sample_proof(p(31), 360);

    DelegationStateOps::upsert_proof_from_dto(old_proof.clone(), 300).expect("store old proof");
    DelegationStateOps::upsert_proof_from_dto(new_proof.clone(), 360).expect("store new proof");

    verify::verify_current_proof(&old_proof).expect("old rotated proof must still verify");
    verify::verify_current_proof(&new_proof).expect("new rotated proof must verify");
}

#[test]
fn latest_proof_prefers_most_recent_keyed_install_for_signing() {
    let old_proof = sample_proof(p(41), 400);
    let new_proof = sample_proof(p(42), 460);

    DelegationStateOps::upsert_proof_from_dto(old_proof, 400).expect("store old proof");
    DelegationStateOps::upsert_proof_from_dto(new_proof.clone(), 460).expect("store new proof");

    let latest = DelegationStateOps::latest_proof_dto().expect("latest proof must exist");
    assert_eq!(
        latest, new_proof,
        "signer selection must use newest keyed proof"
    );
}

#[test]
fn trace_token_trust_chain_stops_at_current_proof_before_signatures() {
    let authority_pid = p(42);
    let self_pid = p(61);
    let token = signed_token_for(p(51), self_pid, 500, 505, 520, 51, 61);

    let (stages, result) = trace_token_trust_chain_with_forced_current_proof_failure(
        &token,
        authority_pid,
        510,
        self_pid,
        DelegationValidationError::ProofMiss.into(),
    );
    let err = result.expect_err("missing local proof must fail before signatures");

    assert_eq!(stages, vec!["structure", "current_proof"]);
    assert_eq!(err.class(), crate::InternalErrorClass::Ops);
    assert!(err.to_string().contains("delegation proof miss"));
}

#[test]
fn trace_token_trust_chain_records_canonical_order_for_valid_token() {
    let authority_pid = p(42);
    let self_pid = p(62);
    let token = signed_token_for(p(52), self_pid, 600, 605, 620, 52, 62);

    DelegationStateOps::upsert_proof_from_dto(token.proof.clone(), 600).expect("store proof");

    let (stages, result) = trace_token_trust_chain(&token, authority_pid, 610, self_pid);
    result.expect("valid token trace must succeed");
    assert_eq!(
        stages,
        vec![
            "structure",
            "current_proof",
            "delegation_signature",
            "token_signature"
        ]
    );

    let verified = DelegatedTokenOps::verify_token(&token, authority_pid, 610, self_pid)
        .expect("valid token must verify");
    let claims = verified.claims.to_dto();
    assert_eq!(claims.sub, token.claims.sub);
    assert_eq!(claims.shard_pid, token.claims.shard_pid);
    assert_eq!(claims.aud, token.claims.aud);
    assert_eq!(claims.scopes, token.claims.scopes);
    assert_eq!(claims.iat, token.claims.iat);
    assert_eq!(claims.exp, token.claims.exp);
    assert_eq!(claims.ext, token.claims.ext);
    assert_eq!(verified.cert.root_pid, token.proof.cert.root_pid);
    assert_eq!(verified.cert.shard_pid, token.proof.cert.shard_pid);
    assert_eq!(verified.cert.issued_at, token.proof.cert.issued_at);
    assert_eq!(verified.cert.expires_at, token.proof.cert.expires_at);
    assert_eq!(verified.cert.aud, token.proof.cert.aud);
    assert_eq!(verified.cert.scopes, token.proof.cert.scopes);
}

#[test]
fn verified_token_claims_preserves_signed_extension_payload() {
    let mut claims = sample_claims();
    claims.ext = Some(b"user_id:toko-123".to_vec());

    let verified = VerifiedTokenClaims::from_dto_ref(&claims);
    let roundtrip = verified.to_dto();

    assert_eq!(roundtrip.ext, claims.ext);
}

#[test]
fn token_signing_hash_binds_extension_payload() {
    let mut claims_a = sample_claims();
    claims_a.ext = Some(b"user_id:toko-123".to_vec());

    let mut claims_b = claims_a.clone();
    claims_b.ext = Some(b"user_id:toko-456".to_vec());

    let cert = sample_proof(p(2), 90).cert;
    let hash_a = crypto::token_signing_hash(&VerifiedTokenClaims::from_dto_ref(&claims_a), &cert)
        .expect("hash_a");
    let hash_b = crypto::token_signing_hash(&VerifiedTokenClaims::from_dto_ref(&claims_b), &cert)
        .expect("hash_b");

    assert_ne!(hash_a, hash_b);
}

#[test]
fn verify_role_attestation_cached_checks_signature_for_resolved_key_id() {
    let key_id = 77;
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id,
        public_key: vec![2; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(100),
        valid_until: None,
    });

    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1, 2, 3],
        key_id,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("invalid signature must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Signature(DelegationSignatureError::AttestationSignatureInvalid(_))
    ));
}

#[test]
fn attestation_keys_sorted_orders_current_before_previous() {
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: 10,
        public_key: vec![10; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(100),
        valid_until: None,
    });
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: 12,
        public_key: vec![12; 33],
        status: AttestationKeyStatus::Current,
        valid_from: Some(120),
        valid_until: None,
    });
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: 11,
        public_key: vec![11; 33],
        status: AttestationKeyStatus::Previous,
        valid_from: Some(90),
        valid_until: Some(110),
    });

    let keys = attestation_keys_sorted();
    let statuses_and_ids: Vec<(AttestationKeyStatus, u32)> = keys
        .into_iter()
        .map(|entry| (entry.status, entry.key_id))
        .collect();

    assert_eq!(
        statuses_and_ids,
        vec![
            (AttestationKeyStatus::Current, 12),
            (AttestationKeyStatus::Current, 10),
            (AttestationKeyStatus::Previous, 11),
        ]
    );
}

#[test]
fn verify_role_attestation_cached_accepts_current_and_previous_keys() {
    let payload = sample_attestation(1);
    let (current_public_key, current_signature) = signing_material(31, &payload);
    let (previous_public_key, previous_signature) = signing_material(41, &payload);

    let current_key_id = 300;
    let previous_key_id = 299;

    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: current_key_id,
        public_key: current_public_key,
        status: AttestationKeyStatus::Current,
        valid_from: Some(100),
        valid_until: None,
    });
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: previous_key_id,
        public_key: previous_public_key,
        status: AttestationKeyStatus::Previous,
        valid_from: Some(90),
        valid_until: Some(300),
    });

    let current_signed = SignedRoleAttestation {
        payload: payload.clone(),
        signature: current_signature,
        key_id: current_key_id,
    };
    let previous_signed = SignedRoleAttestation {
        payload: payload.clone(),
        signature: previous_signature,
        key_id: previous_key_id,
    };

    let verified_current = DelegatedTokenOps::verify_role_attestation_cached(
        &current_signed,
        payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect("current key must verify");
    let verified_previous = DelegatedTokenOps::verify_role_attestation_cached(
        &previous_signed,
        payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect("previous key must verify");

    assert_eq!(verified_current, payload);
    assert_eq!(verified_previous, payload);
}

#[test]
fn verify_role_attestation_cached_rejects_unknown_key_id() {
    let signed = SignedRoleAttestation {
        payload: sample_attestation(1),
        signature: vec![1],
        key_id: 99,
    };
    let err = DelegatedTokenOps::verify_role_attestation_cached(
        &signed,
        signed.payload.subject,
        p(3),
        Some(p(2)),
        150,
        0,
    )
    .expect_err("unknown key_id must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::Validation(DelegationValidationError::AttestationUnknownKeyId {
            key_id: 99
        })
    ));
}
