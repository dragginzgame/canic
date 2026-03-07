use super::*;
use crate::dto::auth::AttestationKeyStatus;
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
        DelegatedTokenOpsError::AttestationSubjectMismatch { .. }
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_audience_mismatch() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(9), Some(p(2)), 150, 0)
        .expect_err("audience mismatch must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::AttestationAudienceMismatch { .. }
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_subnet_mismatch() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(8)), 150, 0)
        .expect_err("subnet mismatch must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::AttestationSubnetMismatch { .. }
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_missing_verifier_subnet() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), None, 150, 0)
        .expect_err("missing verifier subnet must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::AttestationSubnetUnavailable
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_expired_payload() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(2)), 201, 0)
        .expect_err("expired payload must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::AttestationExpired { .. }
    ));
}

#[test]
fn verify_role_attestation_claims_rejects_epoch_floor() {
    let payload = sample_attestation(1);
    let err = verify_role_attestation_claims(&payload, payload.subject, p(3), Some(p(2)), 150, 2)
        .expect_err("epoch floor must fail");
    assert!(matches!(
        err,
        DelegatedTokenOpsError::AttestationEpochRejected { .. }
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
        DelegatedTokenOpsError::AttestationSignatureUnavailable
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
        DelegatedTokenOpsError::AttestationKeyNotYetValid { key_id: 50, .. }
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
        DelegatedTokenOpsError::AttestationKeyExpired { key_id: 51, .. }
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
        DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 2 }
    ));
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
        DelegatedTokenOpsError::AttestationSignatureInvalid(_)
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
        DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 99 }
    ));
}
